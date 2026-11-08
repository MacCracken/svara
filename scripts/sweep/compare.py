#!/usr/bin/env python3
"""Compare svara's observable behaviour between a git ref and the working tree.

Each tree is built with its own toolchain pin, so this works across a cyrius bump.
It compares raw bit patterns, not tolerances. A green test suite proved too little
across a pin bump in naad and goonj, which is why this exists. See README.md.

    scripts/sweep/compare.py 3.6.1
    scripts/sweep/compare.py 3.6.1 --only values,json
    scripts/sweep/compare.py --old-tree /path/to/a/copy
"""
import argparse
import concurrent.futures
import ctypes
import math
import os
import resource
import shutil
import struct
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))

# Keep in step with sec_* and main() in sweep.cyr.
SECTIONS = ["phonemes", "ctx", "pool", "glottal", "tract", "seq", "spectrum", "prosody",
            "traj", "bridge", "misc", "batch", "contours"]
JSON_SECTION = 13
HOSTILE_ENTRIES = 58
HOSTILE_VALUES = ["NaN", "+Inf", "-Inf", "0", "-1", "MAX", "2^63", "1e19", "3e9", "denorm", "-1e19"]
# Entries that print length, stream hash, non-finite count. The rest print raw values.
SUMMARISED = {0, 1, 2, 3, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
              25, 30, 31, 32, 33, 34, 35, 36, 43, 44, 54, 55}
PROBES = ["setup only", "glottal 2M", "formant 1M", "tract 1M", "synthesize_into 500x1024",
          "phoneme /a/ x100", "render_planned x20", "toned x20"]


def limits():
    resource.setrlimit(resource.RLIMIT_AS, (8 << 30, 8 << 30))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def run(binary, *args, timeout=300):
    try:
        r = subprocess.run([binary] + [str(a) for a in args], capture_output=True,
                           timeout=timeout, preexec_fn=limits)
        return r.returncode, r.stdout
    except subprocess.TimeoutExpired:
        return "TIMEOUT", b""


def as_f64(token):
    try:
        return struct.unpack("<d", struct.pack("<Q", int(token, 16)))[0]
    except (ValueError, struct.error):
        return None


def prepare(ref, work, old_tree=None):
    """Materialise both trees and build the harness in each."""
    old, new = os.path.join(work, "old"), os.path.join(work, "new")
    os.makedirs(old)
    os.makedirs(new)
    paths = ["src", "lib", "cyrius.cyml", "cyrius.lock", "VERSION"]
    if old_tree:
        for p in paths:
            src = os.path.join(old_tree, p)
            if os.path.isdir(src):
                shutil.copytree(src, os.path.join(old, p))
            else:
                shutil.copy(src, old)
    else:
        archive = subprocess.run(["git", "-C", REPO, "archive", ref] + paths,
                                 capture_output=True, check=True)
        subprocess.run(["tar", "-x", "-C", old], input=archive.stdout, check=True)
    for p in paths:
        src = os.path.join(REPO, p)
        if os.path.isdir(src):
            shutil.copytree(src, os.path.join(new, p))
        else:
            shutil.copy(src, new)
    for tree, tag in ((old, "old"), (new, "new")):
        for name in ("sweep", "probe"):
            shutil.copy(os.path.join(HERE, name + ".cyr"), tree)
            out = os.path.join(work, f"{name}-{tag}")
            r = subprocess.run(["cyrius", "build", name + ".cyr", out], cwd=tree,
                               capture_output=True, text=True)
            if r.returncode != 0:
                sys.exit(f"build of {name}.cyr failed in the {tag} tree:\n{r.stdout[-800:]}{r.stderr[-800:]}")
        pin = subprocess.run(["cyrius", "version"], cwd=tree, capture_output=True, text=True)
        print(f"{tag}: {pin.stdout.splitlines()[0] if pin.stdout else '?'}")


def compare_values(work):
    total = moved = bad = 0
    print(f"{'section':10s}{'values':>9s}{'moved':>9s}{'max/peak':>11s}  other")
    for i, name in enumerate(SECTIONS):
        ro, oo = run(f"{work}/sweep-old", 0, i)
        rn, on = run(f"{work}/sweep-new", 0, i)
        lo, ln = oo.split(b"\n"), on.split(b"\n")
        if ro != 0 or rn != 0 or len(lo) != len(ln):
            print(f"{name:10s} exit {ro}/{rn}, {len(lo)}/{len(ln)} lines")
            bad += 1
            continue
        count = diff = sign = cls = one_to_zero = 0
        peak = worst = 0.0
        for a, b in zip(lo, ln):
            if a.startswith(b"#") or a in (b"", b"err", b"null", b"code"):
                if a != b:
                    cls += 1
                continue
            fa, fb = as_f64(a), as_f64(b)
            count += 1
            if a == b:
                if fa is not None and math.isfinite(fa):
                    peak = max(peak, abs(fa))
                continue
            diff += 1
            if fa is None or fb is None or math.isnan(fa) != math.isnan(fb) or math.isinf(fa) != math.isinf(fb):
                cls += 1
                continue
            if not math.isfinite(fa):
                continue
            if (fa, fb) == (1.0, 0.0):
                one_to_zero += 1
                continue
            if (fa < 0) != (fb < 0) and fa != 0 and fb != 0:
                sign += 1
            peak = max(peak, abs(fa), abs(fb))
            worst = max(worst, abs(fa - fb))
        rel = worst / peak if peak else 0.0
        notes = []
        if sign:
            notes.append(f"{sign} sign flips")
        if cls:
            notes.append(f"{cls} class changes")
        if one_to_zero:
            notes.append(f"{one_to_zero} 1.0->0.0")
        print(f"{name:10s}{count:9d}{diff:9d}{rel:11.2e}  {', '.join(notes)}")
        total += count
        moved += diff
        bad += sign + cls
    print(f"total {total} values, {moved} moved, {bad} of a kind rounding cannot explain")
    return bad


def compare_json(work):
    ro, oo = run(f"{work}/sweep-old", 0, JSON_SECTION)
    rn, on = run(f"{work}/sweep-new", 0, JSON_SECTION)
    if ro != 0 or rn != 0:
        print(f"json: exit {ro}/{rn}")
        return 1
    if oo == on:
        print(f"json: byte-identical, {len(oo)} bytes in {oo.count(b'\n')} lines")
        return 0
    lo, ln = oo.split(b"\n"), on.split(b"\n")
    diffs = [(a, b) for a, b in zip(lo, ln) if a != b]
    print(f"json: {len(diffs)} lines differ, of {len(lo)}")
    for a, b in diffs[:8]:
        print(f"  old {a[:150].decode(errors='replace')}\n  new {b[:150].decode(errors='replace')}")
    return len(diffs)


def hostile_case(work, case):
    e, v = case
    return case, run(f"{work}/sweep-old", 1, e, v, timeout=40), run(f"{work}/sweep-new", 1, e, v, timeout=40)


def compare_hostile(work):
    cases = [(e, v) for e in range(HOSTILE_ENTRIES) for v in range(len(HOSTILE_VALUES))]
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        results = list(pool.map(lambda c: hostile_case(work, c), cases))
    exits = hash_only = 0
    real, floats, timeouts = [], [], []
    for (e, v), old, new in results:
        name = HOSTILE_VALUES[v]
        if old[0] == "TIMEOUT" or new[0] == "TIMEOUT":
            timeouts.append((e, name, old[0], new[0]))
        if old[0] != new[0]:
            exits += 1
            real.append((e, name, "exit", old[0], new[0]))
            continue
        if old[1] == new[1]:
            continue
        lo, ln = old[1].decode().strip().split("\n"), new[1].decode().strip().split("\n")
        if len(lo) != len(ln):
            real.append((e, name, "length", lo[:4], ln[:4]))
        elif e in SUMMARISED:
            if len(lo) == 3 and lo[0] == ln[0] and lo[2] == ln[2]:
                hash_only += 1
            else:
                real.append((e, name, "structure", lo[:4], ln[:4]))
        else:
            for a, b in zip(lo, ln):
                fa, fb = as_f64(a), as_f64(b)
                if a == b:
                    continue
                if fa is None or fb is None or math.isnan(fa) != math.isnan(fb) or math.isinf(fa) != math.isinf(fb):
                    real.append((e, name, "class", a, b))
                elif math.isfinite(fa):
                    floats.append((e, name, fa, fb))
    print(f"hostile: {len(cases)} cases, {exits} exit-code changes, {hash_only} stream hashes moved "
          f"with length and non-finite count equal, {len(floats)} raw values moved, {len(real)} real")
    print(f"timeouts: {timeouts}")
    for f in floats[:8]:
        print(f"  value  {f}")
    for r in real[:20]:
        print(f"  REAL   {r}")
    return len(real)


def counters(argv):
    """Retired user instructions and cycles for one run. perf_event_paranoid 2 is enough."""
    libc = ctypes.CDLL(None, use_errno=True)
    flags = (1 << 0) | (1 << 5) | (1 << 6) | (1 << 12)   # disabled, no kernel, no hv, on exec
    def attr(config):
        return ctypes.create_string_buffer(struct.pack("IIQQQQQ", 0, 128, config, 0, 0, 0, flags).ljust(128, b"\0"), 128)
    go_r, go_w = os.pipe()
    ready_r, ready_w = os.pipe()
    pid = os.fork()
    if pid == 0:
        os.close(go_w)
        os.close(ready_r)
        os.write(ready_w, b"x")
        os.read(go_r, 1)
        os.dup2(os.open(os.devnull, os.O_WRONLY), 1)
        os.execv(argv[0], argv)
    os.close(go_r)
    os.close(ready_w)
    os.read(ready_r, 1)
    fds = []
    for config in (1, 0):   # instructions, cycles
        fd = libc.syscall(298, attr(config), pid, -1, -1, 0)
        if fd < 0:
            os.write(go_w, b"x")
            os.waitpid(pid, 0)
            raise OSError(ctypes.get_errno(), "perf_event_open")
        fds.append(fd)
    os.write(go_w, b"x")
    os.waitpid(pid, 0)
    values = [struct.unpack("Q", os.read(fd, 8))[0] for fd in fds]
    for fd in fds:
        os.close(fd)
    return values


def compare_counters(work, rounds):
    old, new = f"{work}/probe-old", f"{work}/probe-new"
    try:
        base = {old: counters([old, "0"])[0], new: counters([new, "0"])[0]}
    except OSError as e:
        print(f"counters: unavailable ({e})")
        return 0
    print(f"{'probe':28s}{'old Minstr':>12s}{'new Minstr':>12s}{'ratio':>8s}{'old Mcyc':>10s}{'new Mcyc':>10s}{'ratio':>8s}")
    ratios = []
    for mode in range(1, len(PROBES)):
        ins, cyc = {old: [], new: []}, {old: [], new: []}
        for _ in range(rounds):
            for b in (old, new):
                i, c = counters([b, str(mode)])
                ins[b].append(i - base[b])
                cyc[b].append(c)
        io, inn, co, cn = min(ins[old]), min(ins[new]), min(cyc[old]), min(cyc[new])
        ratios.append(cn / co)
        print(f"{PROBES[mode]:28s}{io / 1e6:12.2f}{inn / 1e6:12.2f}{inn / io:8.4f}{co / 1e6:10.1f}{cn / 1e6:10.1f}{cn / co:8.3f}")
    print(f"cycles geomean {math.exp(sum(map(math.log, ratios)) / len(ratios)):.4f}, min of {rounds} interleaved")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("ref", nargs="?", help="git ref of the old tree, such as a release tag")
    parser.add_argument("--old-tree", help="a directory to use as the old tree instead of a git ref")
    parser.add_argument("--only", default="values,json,hostile,counters", help="comma-separated checks to run")
    parser.add_argument("--rounds", type=int, default=9, help="counter rounds per probe")
    args = parser.parse_args()
    if not args.ref and not args.old_tree:
        parser.error("give a git ref or --old-tree")
    only = args.only.split(",")
    work = tempfile.mkdtemp(prefix="svara-sweep-")
    try:
        prepare(args.ref, work, args.old_tree)
        bad = 0
        if "values" in only:
            bad += compare_values(work)
        if "json" in only:
            bad += compare_json(work)
        if "hostile" in only:
            bad += compare_hostile(work)
        if "counters" in only:
            bad += compare_counters(work, args.rounds)
    finally:
        shutil.rmtree(work, ignore_errors=True)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())

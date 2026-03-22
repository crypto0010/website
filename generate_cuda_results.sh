#!/bin/bash
# DCL-RS CUDA/GPU Results Generator
# Generates comprehensive results file capturing all GPU operations

set -o pipefail

OUTFILE="DCL_RS_CUDA_Results.txt"
CMD_NUM=0
PASS_COUNT=0
FAIL_COUNT=0

run_cmd() {
    CMD_NUM=$((CMD_NUM + 1))
    local cmd="$1"
    echo ""
    echo "CMD $CMD_NUM: $cmd"
    echo "----------------------------------------"
    eval "$cmd" 2>&1
    local rc=$?
    if [ $rc -eq 0 ]; then
        PASS_COUNT=$((PASS_COUNT + 1))
        echo "[EXIT CODE: $rc - OK]"
    else
        FAIL_COUNT=$((FAIL_COUNT + 1))
        echo "[EXIT CODE: $rc - FAILED]"
    fi
    echo "========================================"
    return $rc
}

echo "╔══════════════════════════════════════════════════════════════════╗"
echo "║         DCL-RS CUDA/GPU COMPREHENSIVE RESULTS                  ║"
echo "╚══════════════════════════════════════════════════════════════════╝"
echo ""
echo "Generated: $(date)"
echo "Hostname:  $(hostname)"
echo "OS:        $(uname -a 2>/dev/null || echo 'Windows')"
echo "Rust:      $(rustc --version 2>/dev/null)"
echo "Cargo:     $(cargo --version 2>/dev/null)"
echo ""
echo "Working Directory: $(pwd)"
echo ""

# ============================================================
echo ""
echo "################################################################"
echo "# SECTION 1: SYSTEM & CUDA INFO                               #"
echo "################################################################"

run_cmd "cargo run -p dcl-cli -- gpu --info"

# ============================================================
echo ""
echo "################################################################"
echo "# SECTION 2: ALL dcl-gpu TESTS (with output)                  #"
echo "################################################################"

run_cmd "cargo test -p dcl-gpu -- --nocapture"

# ============================================================
echo ""
echo "################################################################"
echo "# SECTION 3: FULL WORKSPACE TESTS                             #"
echo "################################################################"

run_cmd "cargo test -p dcl-core -p dcl-complexity -p dcl-security -p dcl-crypto -p dcl-hypergraph -p dcl-ramsey -p dcl-zkp -p dcl-gpu"

# ============================================================
echo ""
echo "################################################################"
echo "# SECTION 4: GPU BATCH GCD                                    #"
echo "################################################################"

run_cmd "cargo run -p dcl-cli -- gpu --gcd --count 10000"
run_cmd "cargo run -p dcl-cli -- gpu --gcd --count 50000"
run_cmd "cargo run -p dcl-cli -- gpu --gcd --count 100000"

# ============================================================
echo ""
echo "################################################################"
echo "# SECTION 5: GPU PRIME SIEVE                                  #"
echo "################################################################"

run_cmd "cargo run -p dcl-cli -- gpu --primes --count 10000"
run_cmd "cargo run -p dcl-cli -- gpu --primes --count 50000"
run_cmd "cargo run -p dcl-cli -- gpu --primes --count 100000"

# ============================================================
echo ""
echo "################################################################"
echo "# SECTION 6: GPU BATCH EVOLVE                                 #"
echo "################################################################"

run_cmd "cargo run -p dcl-cli -- gpu --evolve --vertices 5 --count 10000"
run_cmd "cargo run -p dcl-cli -- gpu --evolve --vertices 10 --count 50000"
run_cmd "cargo run -p dcl-cli -- gpu --evolve --vertices 10 --count 100000"
run_cmd "cargo run -p dcl-cli -- gpu --evolve --vertices 10 --count 100000 --modulus 65537"

# ============================================================
echo ""
echo "################################################################"
echo "# SECTION 7: GPU BRUTE-FORCE SEARCH                           #"
echo "################################################################"

run_cmd "cargo run -p dcl-cli -- gpu --bfs --graph path --vertices 4 --count 100000"
run_cmd "cargo run -p dcl-cli -- gpu --bfs --graph path --vertices 6 --count 500000"
run_cmd "cargo run -p dcl-cli -- gpu --bfs --graph cycle --vertices 6 --count 500000"
run_cmd "cargo run -p dcl-cli -- gpu --bfs --graph complete --vertices 4 --count 500000"

# ============================================================
echo ""
echo "################################################################"
echo "# SECTION 8: GPU NIST DATA GENERATION                         #"
echo "################################################################"

run_cmd "cargo run -p dcl-cli -- gpu --nist --vertices 5 --steps 100"
run_cmd "cargo run -p dcl-cli -- gpu --nist --vertices 10 --steps 200"
run_cmd "cargo run -p dcl-cli -- gpu --nist --vertices 10 --steps 200 --modulus 65537"

# ============================================================
echo ""
echo "################################################################"
echo "# SECTION 9: GPU vs CPU BENCHMARK                             #"
echo "################################################################"

run_cmd "cargo run -p dcl-cli -- gpu --benchmark --count 50000"
run_cmd "cargo run -p dcl-cli -- gpu --benchmark --count 100000"
run_cmd "cargo run -p dcl-cli -- gpu --benchmark --count 500000"
run_cmd "cargo run -p dcl-cli -- gpu --benchmark --count 1000000"

# ============================================================
echo ""
echo "################################################################"
echo "# SECTION 10: GPU DEFAULT (ALL DEMOS)                         #"
echo "################################################################"

run_cmd "cargo run -p dcl-cli -- gpu"

# ============================================================
echo ""
echo "################################################################"
echo "# SECTION 11: IGNORED BENCHMARK TESTS                         #"
echo "################################################################"

run_cmd "cargo test -p dcl-gpu -- --ignored --nocapture"

# ============================================================
echo ""
echo "################################################################"
echo "# SECTION 12: SUMMARY                                         #"
echo "################################################################"
echo ""
echo "Total commands run:    $CMD_NUM"
echo "Commands succeeded:    $PASS_COUNT"
echo "Commands failed:       $FAIL_COUNT"
echo ""
echo "Generation completed:  $(date)"
echo ""
echo "╔══════════════════════════════════════════════════════════════════╗"
echo "║                      END OF RESULTS                            ║"
echo "╚══════════════════════════════════════════════════════════════════╝"

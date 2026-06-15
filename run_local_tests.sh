#!/bin/bash
# Local replica of the Matcom grading harness for Windows/MinGW dev.
# Usage: bash run_local_tests.sh <tests_dir>
TESTS_DIR="${1:?usage: run_local_tests.sh <tests_dir>}"
HULK="./hulk.exe"
PASS=0; FAIL=0; FAILED=""

run_ok() {
    local cat="$1" f="$2" name exp got
    name=$(basename "$f" .hulk)
    exp="$(dirname "$f")/$name.expected"
    rm -f output output.exe output.c
    if ! "$HULK" "$f" >/dev/null 2>/tmp/err; then
        FAIL=$((FAIL+1)); FAILED="$FAILED\n[$cat/$name] compile failed: $(head -1 /tmp/err)"; return
    fi
    local bin=output; [ -f output.exe ] && bin=output.exe
    got=$(./$bin 2>/dev/null)
    local e; e=$(sed 's/[[:space:]]*$//' "$exp"); local g; g=$(echo "$got" | sed 's/[[:space:]]*$//')
    if [ "$g" = "$e" ]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); FAILED="$FAILED\n[$cat/$name] expected[$(echo "$e"|tr '\n' '|')] got[$(echo "$g"|tr '\n' '|')]"; fi
}

run_err() {
    local cat="$1" f="$2" want_type="$3" name exit_exp
    name=$(basename "$f" .hulk)
    exit_exp=$(tr -d '[:space:]' < "$(dirname "$f")/$name.exit")
    "$HULK" "$f" >/dev/null 2>/tmp/err; local ec=$?
    local serr; serr=$(cat /tmp/err)
    if [ "$ec" != "$exit_exp" ]; then FAIL=$((FAIL+1)); FAILED="$FAILED\n[$cat/$name] exit want $exit_exp got $ec"; return; fi
    if ! echo "$serr" | grep -q "$want_type"; then FAIL=$((FAIL+1)); FAILED="$FAILED\n[$cat/$name] $want_type missing from stderr"; return; fi
    PASS=$((PASS+1))
}

for f in "$TESTS_DIR"/ok/minimal/*.hulk;  do run_ok "ok/minimal" "$f"; done
for f in "$TESTS_DIR"/ok/types/*.hulk;    do run_ok "ok/types" "$f"; done
for f in "$TESTS_DIR"/ok/oop/*.hulk;      do run_ok "ok/oop" "$f"; done
for f in "$TESTS_DIR"/errors/lexical/*.hulk;   do run_err "errors/lexical" "$f" LEXICAL; done
for f in "$TESTS_DIR"/errors/syntactic/*.hulk; do run_err "errors/syntactic" "$f" SYNTACTIC; done
for f in "$TESTS_DIR"/errors/semantic/*.hulk;  do run_err "errors/semantic" "$f" SEMANTIC; done

echo "============================="
echo "PASS=$PASS FAIL=$FAIL"
echo -e "FAILURES:$FAILED"

#!/usr/bin/env bash
# Exercise deadline processing and graceful restarts in the disposable API cluster.

deadline_worker_demo_python() {
  TT_DEADLINE_REEVALUATION_ACCEPTANCE=1 \
    TT_DEADLINE_API_BASE_URL="$BASE_URL" \
    TT_DEADLINE_API_TOKEN="$RECOVERY_TOKEN" \
    TT_DEADLINE_API_WORK_DIR="$WORK_DIR" \
    python3 -B "$REPO_ROOT/scripts/api-deadline-reevaluation-demo.py" "$1"
}

deadline_worker_demo_stop() {
  local signal="$1" stopped_pid="$SERVER_PID" exit_status=0
  case "$signal" in TERM|INT) ;; *) return 2 ;; esac
  test -n "$stopped_pid"
  kill -0 "$stopped_pid"
  kill "-$signal" "$stopped_pid"
  wait "$stopped_pid" || exit_status=$?
  SERVER_PID=""
  if [ "$exit_status" -ne 0 ]; then
    printf 'serve SIG%s exited %s; expected graceful exit 0\n' "$signal" "$exit_status" >&2
    return 1
  fi
  printf 'serve SIG%s graceful exit 0 confirmed\n' "$signal"
}

deadline_worker_demo() {
  local url="$1" directory="$2"
  deadline_worker_demo_python capture
  deadline_worker_demo_stop TERM
  migration_demo_start "$url" "$directory" worker-after-term
  deadline_worker_demo_python restarted
  deadline_worker_demo_stop INT
  migration_demo_start "$url" "$directory" worker-after-int
  deadline_worker_demo_python verify
}

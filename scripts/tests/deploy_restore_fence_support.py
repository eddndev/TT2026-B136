"""Private deployment fixture and side-effect sentinels for restore admission."""
from contextlib import ExitStack
import json
from unittest.mock import patch

from test_deploy_crl_fence import CrlFenceTests


OPERATION = "10000000-0000-4000-8000-000000000001"
OTHER_OPERATION = "20000000-0000-4000-8000-000000000002"


def setup(case):
    fixture = CrlFenceTests()
    fixture.setUp()
    case.addCleanup(fixture.doCleanups)
    fixture.fence.unlink()
    case.root = fixture.root
    case.current = fixture.current
    case.previous = fixture.previous
    case.next = fixture.next
    case.marker = fixture.root / "maintenance/restore/active.json"


def put_marker(case):
    case.marker.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    case.marker.write_text(json.dumps({"operation_id": OPERATION}) + "\n")
    case.marker.chmod(0o600)


def denied_before(case, action, edges):
    with ExitStack() as stack:
        sentinels = [stack.enter_context(patch.object(
            owner, name, side_effect=AssertionError("effect reached before restore guard")))
            for owner, name in edges]
        with case.assertRaisesRegex(RuntimeError, "restore|maintenance"):
            action()
        for sentinel in sentinels:
            sentinel.assert_not_called()

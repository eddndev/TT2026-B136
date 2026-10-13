"""Do not mutate a CA whose portal operation still needs reconciliation."""
import stat


def guard(root, allowed=None):
    from crl_journal import load
    ca = root / "data/ca"
    for group in ("operations", "revocations", "publications"):
        directory = ca / group
        try:
            info = directory.lstat()
        except FileNotFoundError:
            continue
        if not stat.S_ISDIR(info.st_mode):
            raise ValueError("CA operation directory must be a directory")
        for operation in directory.iterdir():
            if not stat.S_ISDIR(operation.lstat().st_mode):
                raise ValueError("CA operation must be a directory")
            if allowed == (group, operation.name):
                continue
            record = load(operation / "journal.json")
            if not isinstance(record.get("receipt"), dict) or not record["receipt"]:
                raise RuntimeError("A pending CA portal operation must be reconciled before another mutation")

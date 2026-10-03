"""Bound private restore command output and reap only the process group created."""
import math
import os
import selectors
import signal
import subprocess
import time


def run(arguments, *, env=None, timeout=10, maximum=65536, text=False,
        check=True, capture_output=True, cwd=None):
    if (type(maximum) is not int or not 0 < maximum <= 8 * 1024 * 1024
            or type(timeout) not in (int, float) or not math.isfinite(timeout)
            or not 0 < timeout <= 300 or not capture_output or not check):
        raise ValueError("restore command limits are invalid")
    process = None
    completed = False
    reaped = False
    try:
        process = subprocess.Popen(
            [str(value) for value in arguments], env=env, cwd=cwd,
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            start_new_session=True)
        deadline = time.monotonic() + timeout
        output = bytearray()
        sizes = {process.stdout: 0, process.stderr: 0}
        with selectors.DefaultSelector() as selector:
            for stream in sizes:
                os.set_blocking(stream.fileno(), False)
                selector.register(stream, selectors.EVENT_READ)
            while selector.get_map():
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError
                ready = selector.select(remaining)
                if not ready:
                    raise TimeoutError
                for key, _ in ready:
                    stream = key.fileobj
                    value = os.read(stream.fileno(), min(65536, maximum - sizes[stream] + 1))
                    if not value:
                        selector.unregister(stream)
                        continue
                    sizes[stream] += len(value)
                    if sizes[stream] > maximum:
                        raise ValueError("restore command output exceeded its boundary")
                    if stream is process.stdout:
                        output.extend(value)
            result = process.wait(timeout=max(0.001, deadline - time.monotonic()))
            reaped = True
        if result != 0:
            raise RuntimeError("restore command did not complete")
        value = bytes(output)
        if text:
            value = value.decode("utf-8")
        completed = True
        return subprocess.CompletedProcess(arguments, result, stdout=value, stderr="" if text else b"")
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError):
        raise RuntimeError("restore external command failed") from None
    finally:
        if process is not None:
            try:
                if not completed and not reaped:
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    process.wait(timeout=5)
            except (OSError, subprocess.SubprocessError):
                raise RuntimeError("restore process cleanup did not complete") from None
            finally:
                for stream in (process.stdout, process.stderr):
                    if stream is not None:
                        stream.close()

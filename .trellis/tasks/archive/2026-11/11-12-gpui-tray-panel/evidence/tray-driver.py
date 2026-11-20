"""Owned HWNDs only. Start this script through private_desktop.py on a non-input desktop."""

from pathlib import Path
import os, sys, time, subprocess, ctypes as c, json, traceback
from ctypes import wintypes as w
import win32gui, win32process, win32ui
from PIL import Image

user32 = c.WinDLL("user32")
user32.SetProcessDpiAwarenessContext(c.c_void_p(-4))
user32.GetThreadDesktop.restype = w.HANDLE
desktop = user32.GetThreadDesktop(c.windll.kernel32.GetCurrentThreadId())
name = c.create_unicode_buffer(256)
length = w.DWORD()
user32.GetUserObjectInformationW(
    w.HANDLE(desktop), 2, name, c.sizeof(name), c.byref(length)
)
assert name.value.startswith("DevSweepReview_"), name.value
root = Path.cwd()
binary = Path(sys.argv[1]).resolve()
scene = sys.argv[2]
output = Path(sys.argv[3]).resolve()
output.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, DEVSWEEP_NATIVE_TRAY_RECEIPT=scene, RUST_MIN_STACK="16777216")
for key_code in ("TEMP", "TMP", "APPDATA", "LOCALAPPDATA"):
    directory = output / key_code.lower()
    directory.mkdir(exist_ok=True)
    env[key_code] = str(directory)
log = (output / "app.log").open("w", encoding="utf8")
application = subprocess.Popen(
    [
        str(binary),
        "--exact",
        "view::tray_panel::receipt::native_tray_receipt",
        "--ignored",
        "--nocapture",
    ],
    stdout=log,
    stderr=subprocess.STDOUT,
    env=env,
    creationflags=subprocess.CREATE_NO_WINDOW,
)
rows = []


def record(event, **data):
    rows.append(dict(event=event, **data))
    (output / "observations.json").write_text(
        json.dumps(rows, indent=2, ensure_ascii=False), encoding="utf8"
    )


def owned(title=None, cls=None):
    windows = []

    def check(hwnd, _):
        if (
            win32process.GetWindowThreadProcessId(hwnd)[1] == application.pid
            and (title is None or win32gui.GetWindowText(hwnd) == title)
            and (cls is None or win32gui.GetClassName(hwnd) == cls)
        ):
            windows.append(hwnd)
        return True

    win32gui.EnumWindows(check, None)
    return next(iter(windows), 0)


def wait(fn, label, seconds=8):
    due = time.monotonic() + seconds
    while time.monotonic() < due:
        result = fn()
        if result:
            return result
        time.sleep(0.06)
    raise RuntimeError(label)


def capture(hwnd, label):
    assert win32process.GetWindowThreadProcessId(hwnd)[1] == application.pid
    time.sleep(0.45)
    rect = win32gui.GetWindowRect(hwnd)
    width, height = rect[2] - rect[0], rect[3] - rect[1]
    window_dc = win32gui.GetWindowDC(hwnd)
    source_dc = win32ui.CreateDCFromHandle(window_dc)
    memory_dc = source_dc.CreateCompatibleDC()
    bitmap = win32ui.CreateBitmap()
    bitmap.CreateCompatibleBitmap(source_dc, width, height)
    memory_dc.SelectObject(bitmap)
    try:
        assert user32.PrintWindow(w.HWND(hwnd), w.HDC(memory_dc.GetSafeHdc()), 2)
        image = Image.frombuffer(
            "RGB", (width, height), bitmap.GetBitmapBits(True), "raw", "BGRX", 0, 1
        )
        image.save(output / (label + ".png"))
        assert image.getextrema()[0][0] != image.getextrema()[0][1], (
            label,
            image.getextrema(),
        )
        record(
            label, size=image.size, dpi=user32.GetDpiForWindow(w.HWND(hwnd)), rect=rect
        )
    finally:
        memory_dc.DeleteDC()
        source_dc.DeleteDC()
        win32gui.ReleaseDC(hwnd, window_dc)
        win32gui.DeleteObject(bitmap.GetHandle())


def post_key(hwnd, key_code):
    win32gui.PostMessage(hwnd, 0x100, key_code, 1)
    win32gui.PostMessage(hwnd, 0x101, key_code, 0xC0000001)


def send(code):
    win32gui.PostMessage(receiver, 0x8000 + 19, (860 << 16) | 1260, (1 << 16) | code)


def open_panel(keyboard=False):
    send(1025 if keyboard else 1024)
    return wait(lambda: owned("DevSweep Status"), "panel did not open")


def menu():
    send(0x7B)
    return wait(lambda: owned(cls="#32768"), "native menu did not open")


def menu_choose(index):
    # Standard menu message loop consumes the queued key for the owning thread.
    menu_window = owned(cls="#32768")
    assert menu_window
    win32gui.PostMessage(menu_window, 0x102, ord("x" if index else "o"), 1)
    time.sleep(0.2)


try:
    main = wait(lambda: owned("DevSweep"), "main did not open", 20)
    receiver = wait(lambda: owned("DevSweep Tray Receiver"), "receiver missing")
    time.sleep(0.7)
    capture(main, "status")

    def sampler_logs():
        return [
            s
            for s in (output / "app.log").read_text(errors="replace").splitlines()
            if s.startswith("tray-sampler ")
        ]

    panel = open_panel()
    capture(panel, "panel")
    logs = sampler_logs()
    starts = [s for s in logs if s.startswith("tray-sampler start")]
    assert len(starts) == 1, logs
    record(
        "shared_sampler", starts=starts, sample=next(s for s in logs if "sample " in s)
    )
    if scene == "en:light":
        win32gui.SendMessage(panel, 0x6, 0, main)
        wait(lambda: not owned("DevSweep Status"), "deactivation did not close popup")
        record("outside_focus_closed")
        panel = open_panel()

    post_key(panel, 27)
    wait(lambda: not owned("DevSweep Status"), "escape did not close popup")
    record("escape_closed")
    win32gui.ShowWindow(main, 6)
    wait(lambda: win32gui.IsIconic(main), "did not minimize")
    wait(lambda: any(" stop " in s for s in sampler_logs()), "sampler did not stop")
    stopped = sampler_logs()
    time.sleep(0.8)
    assert sampler_logs() == stopped
    record(
        "neither_visible",
        stops=[s for s in stopped if " stop " in s],
        no_new_samples_ms=800,
    )
    panel = open_panel(True)
    record(
        "keyboard_panel_while_minimized",
        panel_shown=bool(win32gui.IsWindowVisible(panel)),
    )
    post_key(panel, 27)
    wait(lambda: not owned("DevSweep Status"), "escape while minimized")
    taskbar = win32gui.RegisterWindowMessage("TaskbarCreated")
    win32gui.SendMessage(receiver, taskbar, 0, 0)
    wait(
        lambda: (
            (output / "app.log").read_text(errors="replace").count("tray-shell version")
            >= 2
        ),
        "missing reregister",
    )
    record(
        "taskbar_created",
        message=taskbar,
        registered=(output / "app.log")
        .read_text(errors="replace")
        .count("tray-shell version"),
    )
    menu_window = menu()
    capture(menu_window, "menu")
    menu_choose(0)
    wait(
        lambda: win32gui.IsWindowVisible(main) and not win32gui.IsIconic(main),
        "open command did not restore",
    )
    record("menu_open_restored")
    if env.get("DEVSWEEP_TRAY_RESIDENT") == "1":
        win32gui.PostMessage(main, 0x10, 0, 0)
        wait(lambda: not win32gui.IsWindowVisible(main), "did not hide on close")
        record("closed_to_tray", running=application.poll() is None)
        panel = open_panel()
        post_key(panel, 27)
        wait(lambda: not owned("DevSweep Status"), "hidden popup escape")
        menu_window = menu()
        menu_choose(0)
        wait(lambda: win32gui.IsWindowVisible(main), "reopen after hide")
        record("reopened_after_close")
    if env.get("DEVSWEEP_TRAY_AWAKE_REVIEW") == "1":
        panel = open_panel()
        time.sleep(0.55)
        width, height = win32gui.GetClientRect(panel)[2:]
        position = (int(height * 0.882) << 16) | int(width * 0.5)
        for msg, wparam in [(0x200, 0), (0x201, 1), (0x202, 0)]:
            win32gui.PostMessage(panel, msg, wparam, position)
            time.sleep(0.07)
        wait(
            lambda: not owned("DevSweep Status"), "awake action did not return to main"
        )
        inspect = Path(__file__).resolve().parent / "tray-read-uia.ps1"
        win32gui.PostMessage(main, 0x200, 0, 6 | (6 << 16))
        read = subprocess.run(
            [
                "powershell",
                "-NoProfile",
                "-File",
                str(inspect),
                str(main),
                str(application.pid),
            ],
            capture_output=True,
        )
        assert read.returncode == 0, (read.stdout, read.stderr)
        capture(main, "awake-review")
        record(
            "awake_opens_existing_review",
            control="host-action-confirm",
            confirmed=False,
        )
    if env.get("DEVSWEEP_TRAY_DEFAULT_EXIT") == "1":
        win32gui.PostMessage(main, 0x10, 0, 0)
    else:
        menu_window = menu()
        menu_choose(1)
    exit_code = application.wait(5)
    record(
        "exit",
        code=exit_code,
        icon_removed="tray-shell delete"
        in (output / "app.log").read_text(errors="replace"),
    )
    assert exit_code == 0
except:
    record("failure", detail=traceback.format_exc())
    raise
finally:
    if application.poll() is None:
        application.terminate()
        application.wait(4)
    log.close()

# Operator checklist (R5 operator rows)

These rows need a person at the Windows shell. The agent does not perform
them. Use the release executable recorded in `identity.json`
(`target/release/devsweep-desktop.exe`). Use only a disposable test folder
and a startup item that you choose. Record `pass`, `fail`, or `not run` in
the Result column, and write what you saw for each `fail`.

| ID                        | Steps                                                                                                           | Expected result                                                                                                                     | Result |
| ------------------------- | --------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------ |
| OP-1 Tray icon            | Start the app. Find the DevSweep icon in the notification area.                                                 | One DevSweep icon is shown. Its tooltip is `DevSweep`.                                                                              |        |
| OP-2 HUD show/hide        | Left-click the tray icon. Left-click it again.                                                                  | The first click shows a small HUD window near the tray with CPU and memory values. The second click hides it.                       |        |
| OP-3 HUD tooltip          | Show the HUD, wait 4 s, and point to the tray icon.                                                             | The tooltip shows the CPU and memory values. After the HUD is hidden, the tooltip is `DevSweep` again.                              |        |
| OP-4 Tray menu            | Right-click the tray icon.                                                                                      | The menu shows `Open DevSweep` and `Quit` (`打开 DevSweep` and `退出` in Chinese).                                                  |        |
| OP-5 Open from tray       | Minimize the main window. Select `Open DevSweep`.                                                               | The main window is restored and gets focus.                                                                                         |        |
| OP-6 Quit from tray       | Show the HUD, then select `Quit`. Open Task Manager > Details after 5 s.                                        | No `devsweep-desktop.exe` process remains. No `msedgewebview2.exe` process with a DevSweep command line remains.                    |        |
| OP-7 Startup toggle       | Software > View startup items. Turn off one item that you choose. Open Task Manager > Startup apps.             | Task Manager shows the same item as Disabled. Turn it on again in DevSweep; Task Manager shows Enabled. History shows both changes. |        |
| OP-8 Explorer reveal      | Analyze a disposable folder. Right-click a row and select the reveal action.                                    | File Explorer opens the parent folder with that item selected.                                                                      |        |
| OP-9 Recycle Bin move     | In the same disposable folder, right-click a row and select the move to Recycle Bin action. Confirm the dialog. | The item leaves the list and the folder. It appears in the Recycle Bin.                                                             |        |
| OP-10 Recycle Bin restore | Restore the item from OP-9 in the Recycle Bin.                                                                  | The item is back at its original path with the same contents.                                                                       |        |
| OP-11 Refusal             | Try the Recycle Bin action on `C:\Windows` or your profile root.                                                | The action is refused with a reason. Nothing moves.                                                                                 |        |
| OP-12 Real window widths  | Resize the main window to its minimum size (900 x 600) and to full screen.                                      | No clipped action, no horizontal page scroll, and the capsule stays readable.                                                       |        |
| OP-13 Clean review | Clean > Scan. Open the review. Select one item. Run the dry run. Open the confirmation dialog, then select Cancel. | The dry run shows a digest. The dialog names the item. After Cancel, nothing moves and History shows no execute record. | |

Out of scope for the operator: running the NSIS installer, changing the
Windows display scale, running a winget upgrade, and running a Clean execute
against real data.

# Native IME capability observation

`StatefulAppBuilder::on_native_ime_adapter_observation` is an opt-in,
one-shot diagnostic callback. The generic native runtime emits one
`NativeImeAdapterObservation` only after a primary or auxiliary window has
completed its existing admission path. The callback does not own focus,
composition, commands, or input routing.

| Created window handle | Composition events | Candidate placement | Matching-key suppression |
| --- | --- | --- | --- |
| AppKit | `SupportedByWinit` | `FullCursorAreaByWinit` | `VerifiedWinitAppKit` |
| Win32 | `SupportedByWinit` | `FullCursorAreaByWinit` | unavailable (`Win32`) |
| Wayland | `SupportedByWinit` | `FullCursorAreaByWinit` | unavailable (`Wayland`) |
| X11 | `SupportedByWinit` | `PositionOnlyByWinit` | unavailable (`X11`) |
| Unknown | unavailable (`UnknownBackend`) | unavailable (`UnknownBackend`) | unavailable (`UnknownBackend`) |
| Handle error | unavailable (`WindowHandleUnavailable`) | unavailable (`WindowHandleUnavailable`) | unavailable (`WindowHandleUnavailable`) |

The composition and candidate fields describe the actual locked-Winit adapter,
and default to unavailable when the adapter cannot be identified. In the pinned
`winit 0.30.12` source, `platform_impl/macos/view.rs` lines 386-489 calls
`interpretKeyEvents`, emits `Ime::Preedit`/`Ime::Commit`, and suppresses the
matching `KeyboardInput` unless Winit explicitly forwards it. This is the
only suppression outcome reported as verified. `Window::set_ime_cursor_area`
documents complete cursor-area support except on X11, where only position is
supported. Other matching-key suppression outcomes remain unavailable without
changing generic IME event delivery.

Manual acceptance remains OPT-1378 and is not performed by this diagnostic
slice. On macOS, test Japanese and Chinese input in an admitted text field:
complete one composition with one commit and confirm the same keystroke does
not create a duplicate insertion. Repeat after opening an auxiliary window.

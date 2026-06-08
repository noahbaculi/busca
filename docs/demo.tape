# busca demo — core interactive flow
#
# One-time tooling setup:
#   brew install vhs ttyd ffmpeg   # the recorder, the headless terminal it drives, and the GIF encoder
#
# Regenerate (run from the repo root so $PWD/target/release resolves):
#   cargo build --release && vhs docs/demo.tape

Output docs/demo.gif

# A floating macOS-style terminal window: traffic-light bar, rounded corners,
# the classic Menlo face, and a cursor that blinks like a live session.
Set Shell "bash"
Set FontFamily "Menlo"
Set FontSize 18
Set Width 1340
Set Height 850
Set Padding 24
Set BorderRadius 12
Set WindowBar "Colorful"
Set Theme "Catppuccin Mocha"
Set CursorBlink true
Set TypingSpeed 55ms

# Hidden setup: a colored two-segment prompt, and make the release binary callable as plain `busca`.
Hide
Type `export PS1="\[\e[36m\]busca\[\e[0m\] \[\e[32m\]❯\[\e[0m\] "`
Enter
Type `export PATH="$PWD/target/release:$PATH"`
Enter
Type "clear"
Enter
Show

# Type the way a person does: the command name first, the flags a little quicker, a beat before running.
Sleep 1s
Type@90ms "busca "
Type@45ms "--ref-file-path sample_dir_mix/file_5.py --search-path sample_dir_mix"
Sleep 400ms
Enter

# Wait for the inquire picker to actually render rather than guessing a fixed delay.
Wait+Screen@10s /Select a file to compare/
Sleep 2s

# Move from file_5.py (100.0%) down to file_5v2.py (97.2%) and select it.
Down
Sleep 250ms
Down
Sleep 500ms
Down
Sleep 650ms
Up
Sleep 350ms
Up
Sleep 2s
Enter

# Hold on the colored line-diff before the recording ends.
Sleep 7s

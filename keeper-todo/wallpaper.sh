#!/bin/sh

set -eu

# If you try to change the wallpaper to a file with the same name, it seems like
# it uses a cached version. Thus, we just make a new file with a different name
# and delete the old one.

DIR="$HOME/code/rust/keeper/keeper-todo"

# Generate new wallpaper filename
WALLPAPER_FILE="$DIR/wallpapers/wallpaper-$(date "+%Y-%m-%d-%H-%M-%S").png"

# Create the wallpaper image
~/.local/bin/keeper-todo render today "$WALLPAPER_FILE"

# Set the new wallpaper
automator -i "$WALLPAPER_FILE" "$DIR/wp.workflow"


# dmgbuild settings: drag-to-Applications window. Run as
#   dmgbuild -s mac/dmg/settings.py -D app=<path/App.app> -D bg=<background.tiff> "<Volume>" out.dmg
# dmgbuild writes the Finder layout (.DS_Store) itself, so no Finder/AppleScript is needed on CI.
import os.path

app = defines["app"]  # noqa: F821 (provided by dmgbuild)
name = os.path.basename(app)
format = "UDZO"
filesystem = "HFS+"
files = [app]
symlinks = {"Applications": "/Applications"}
background = defines["bg"]  # noqa: F821
icon_size = 128
text_size = 13
show_status_bar = show_tab_view = show_toolbar = show_pathbar = show_sidebar = False
window_rect = ((200, 160), (660, 400))
default_view = "icon-view"
show_icon_preview = False
arrange_by = None
icon_locations = {name: (180, 180), "Applications": (480, 180)}

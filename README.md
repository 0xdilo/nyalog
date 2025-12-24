```
                         .__
   ____ ___.__._____    |  |   ____   ____
  /    <   |  |\__  \   |  |  /  _ \ / ___\
 |   |  \___  | / __ \_ |  |_(  <_> ) /_/  >
 |___|  / ____|(____  / |____/\____/\___  /
      \/\/          \/             /_____/

      /\_____/\
     /  o   o  \
    ( ==  ^  == )
     )         (
    (           )
   ( (  )   (  ) )
  (__(__)___(__)__)
```

# nyalog

a smol keylogger for linux :3

## features

- auto-detects keyboard layout (hyprland, sway, x11, vconsole)
- logs to `~/.config/nyalog/YYYY-MM-DD.log`
- one file per day, clean output
- works on wayland & x11
- handles shift, caps lock, special keys

## install

from git (408K):
```bash
cargo install --git https://github.com/0xdilo/nyalog
```

with upx compression (164K):
```bash
git clone https://github.com/0xdilo/nyalog
cd nyalog
make install
```

add yourself to input group (so no sudo needed):
```bash
sudo usermod -aG input $USER
```
then log out & back in

## usage

```bash
nyalog
```

or with sudo if not in input group:
```bash
sudo nyalog
```

override layout manually:
```bash
NYALOG_LAYOUT=ch:de nyalog    # swiss german
NYALOG_LAYOUT=de nyalog       # german
NYALOG_LAYOUT=us nyalog       # us english
```

## output

logs look like:
```
hello world :3
this is a test[BS][BS][BS][BS]cool
arrow keys: [UP][DOWN][LEFT][RIGHT]
```

## license

do whatever u want with it lol

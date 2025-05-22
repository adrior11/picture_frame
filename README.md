# picture_frame

```
# /etc/systemd/system/pictureframe-backend.service
[Unit]
Description=Picture-Frame API
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=pi
WorkingDirectory=/home/pi/Documents/picture_frame
ExecStart=/home/pi/Documents/picture_frame/backend
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```
# /etc/systemd/system/pictureframe-display.service
[Unit]
Description=Picture-Frame Image Slideshow
After=pictureframe-backend.service user-runtime-dir@%U.service network-online.target
Requires=pictureframe-backend.service

[Service]
Type=simple
User=pi
Environment=DISPLAY=:0
Environment=SDL_VIDEODRIVER=x11
Environment=XAUTHORITY=/home/pi/.Xauthority
Environment=XDG_RUNTIME_DIR=/run/user/%U
WorkingDirectory=/home/pi/Documents/picture_frame
ExecStart=/home/pi/Documents/picture_frame/display
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

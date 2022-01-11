# webtail-rs

"tail -f" as a simple web service, written in rust

```
>> ./webtail-rs --help
webtail-rs 0.1.0
Make "tail -f" as a web service

USAGE:
    webtail-rs [OPTIONS] --filepath <FILEPATH>

OPTIONS:
    -f, --filepath <FILEPATH>    Path of log file
    -h, --help                   Print help information
        --passwd <PASSWD>        Password of basic auth [default: webtail]
        --port <PORT>            TCP port to bind [default: 3000]
        --username <USERNAME>    Username of basic auth [default: webtail]
    -V, --version                Print version information
``````
```

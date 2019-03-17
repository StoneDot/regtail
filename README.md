# regtail
Regex base tail written in Rust.

[![Build Status](https://travis-ci.org/StoneDot/regtail.svg?branch=master)](https://travis-ci.org/StoneDot/regtail)
[![Build status](https://ci.appveyor.com/api/projects/status/4ms5u2h1q0pspovx/branch/master?svg=true)](https://ci.appveyor.com/project/StoneDot/regtail/branch/master)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

## Documentation quick links

* [Why regtail?](#why-regtail)
* [Installation](#installation)

# Why regtail?
`tail -F` is very common way to monitor log files.
Although it requires specify the monitored files before it's launched as below.

```bash
> ls
log.20190101 log.20190102
> tail -F log.*
==> log.20190101 <==
This is log.20190101

==> log.20190102 <==
This is log.20190102
```

It seems to be sufficient to monitor all log files. But actually this IS NOT
the sufficient way as follows:

```bash
term1 > ls
log.20190101 log.20190102
term1 > tail -F log.*
==> log.20190101 <==
This is log.20190101

==> log.20190102 <==
This is log.20190102
term2 > echo "This is log.20190103" > log.20190103
term1 > # No output on term1
```

Newly created file is not monitored at all!

This problem is solved by regtail! You just run regtail with no arguments as follows:

```bash
term1 > ls
log.20190101 log.20190102
term1 > regtail
==> log.20190101 <==
This is log.20190101

==> log.20190102 <==
This is log.20190102
term2 > echo "This is log.20190103" > log.20190103
term1 > # term1 output is below

==> log.20190103 <==
This is log.20190103
```

Moreover you can specify target files with regular expression as follow:

```bash
> ls
error.20180101 error.20190101 error.20190102 log.20190101 log.20190102
> regtail 'error\.\d{4}0101'
==> error.20180101 <==
This is error.20180101

==> error.20190101 <==
This is error.20190101
```

Regtail is the perfect way to monitor your log files in all situation, isn't it?

## Installation
### Homebrew
```bash
brew tap StoneDot/regtail
brew install regtail
```

### Binary
```bash
# Linux x86_64
wget https://github.com/StoneDot/regtail/releases/download/v0.0.2/regtail-v0.0.2-x86_64-unknown-linux-gnu.tar.gz
tar zxf regtail-v0.0.2-x86_64-unknown-linux-gnu.tar.gz
cd regtail-v0.0.2-x86_64-unknown-linux-gnu
sudo cp regtail /usr/local/bin
```

### Source build
```bash
wget https://github.com/StoneDot/regtail/archive/v0.0.2.tar.gz
tar zxf v0.0.2.tar.gz
cd regtail-0.0.2
cargo install --root $HOME --path .
export PATH="$HOME/bin:$PATH"
```
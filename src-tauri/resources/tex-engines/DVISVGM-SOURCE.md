# dvisvgm source availability

The GPL build of TeXKey contains an unmodified universal slice of the dvisvgm
executable distributed with TeX Live 2026:

- dvisvgm version: 3.6
- architectures: x86_64, arm64
- vendored from: `/usr/local/texlive/2026/bin/universal-darwin/dvisvgm`
- bundled SHA-256:
  `ffad3ec96961676efe4fd811b8b2d92832b6a513f2a3cf878d702aac5e8a8871`

dvisvgm is licensed under GNU GPL version 3 or, at the recipient's option,
any later version. The complete corresponding TeX Live 2026 program sources,
including the build system and the library sources used by the TeX Live
binary build, are available from:

- https://ftp.tug.org/historic/systems/texlive/2026/
- https://github.com/TeX-Live/texlive-source/tree/branch2026

The upstream dvisvgm 3.6 source is also available from:

- https://github.com/mgieseki/dvisvgm/tree/3.6

These source locations must remain available alongside any public download of
the TeXKey distribution that contains dvisvgm. A redistributor is responsible
for satisfying the source-conveyance requirements of GPLv3 section 6.

The BSD build of TeXKey does not contain dvisvgm and imposes no GPL
obligations; it invokes the copy installed by the user.

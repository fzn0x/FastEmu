# Security

## Reporting a vulnerability

Report security problems privately, through GitHub's
[private vulnerability reporting](https://github.com/fzn0x/FastEmu/security/advisories/new),
not in a public issue. Say what an attacker could do, and how to reproduce
it. If a crafted JAR triggers it, attach the file to the report. You will
hear back within a week, and be credited in the fix unless you prefer not to.

## Supported versions

FastEmu has no stable release yet. Fixes go into the newest release and
into main. Once there are several release lines, this table tells which
ones still get fixes.

| Version | Supported |
| --- | --- |
| The newest release | Yes |
| Older releases | No: update to the newest |

## What FastEmu protects

A Java ME game is code from someone else, often a file of unknown origin
from an old download site. FastEmu runs it in a virtual machine and treats
it as untrusted.

- **The sandbox.** A game reaches the outside only through the MIDP and
  JSR APIs FastEmu implements. It cannot read or write your files outside
  its own record stores and the files you give it, start programs, or touch
  the memory of FastEmu or of another game.
- **Each game on its own.** The games in one window do not share memory,
  record stores or threads. A crash or a runaway loop in one stays in that
  game.
- **The network.** A game opens connections only as far as the network
  policy allows, which you set per game. Reaching the network without that
  permission is a bug.
- **JAR and JAD files.** FastEmu reads them as hostile input: a damaged or
  crafted archive, class file or manifest must produce an error message,
  not a crash, a hang or unbounded memory use.

## In scope

- A game escaping its sandbox: reading or writing outside its allowed files,
  running native code, or reaching another game's data.
- A crafted JAR, JAD or class file that crashes FastEmu, makes it hang or
  makes it use memory without a limit.
- A way around the network policy.
- A way for one game to take down the games next to it.

## Not in scope

- A game that does not run or looks wrong. Report it as a
  [game report](https://github.com/fzn0x/FastEmu/issues/new?template=game_report.yml).
- A game that harms itself, such as one that fills its own record store.
- Problems that need you to give a game permissions you were warned about.
- Findings in dependencies that FastEmu's use does not reach.

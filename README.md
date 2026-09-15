# loglint

A validating parser and pretty printer for a plain-text structured log line
format. Point it at a file, get either a hard error on the first bad line
(default) or a best-effort normalized rewrite (`--lenient`).

## The problem

Most in-house log formats are "structured" only until someone hand-writes a
log line, or a library changes its message format, or a field value happens
to contain a space nobody accounted for. Tools that consume these logs
(grep pipelines, dashboards, alerting) usually just silently misparse the
bad lines instead of telling you they exist. This tool draws a hard line:
either a file conforms to the grammar, or you find out exactly which line
and why it doesn't.

## The format

```
TIMESTAMP LEVEL MODULE: MESSAGE [| FIELD FIELD ...]
```

- `TIMESTAMP` is `YYYY-MM-DDTHH:MM:SS.mmmZ`, always UTC, always millisecond
  precision, always exactly 24 characters.
- `LEVEL` is one of `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`.
- `MODULE` is one or more `.`-separated identifiers, followed by `:`.
- `MESSAGE` is free text, and may not itself contain the literal ` | `
  sequence (that is reserved to introduce the field list).
- `FIELDS`, if present, is a space-separated list of `key=value` pairs.
  A value containing whitespace or a double quote must be wrapped in double
  quotes, with `\"` and `\\` as the only escapes.

Example, well-formed:

```
2026-09-16T10:23:01.123Z INFO auth.session: user logged in | user_id=42 ip=10.0.0.5
2026-09-16T10:23:02.500Z WARN cache.evict: evicting stale entry | key=session:9f2c reason="ttl expired"
```

## Strict by default, `--lenient` as the escape hatch

By default the parser treats any deviation from the grammar above as fatal:
a bad timestamp, an unrecognized level, a missing colon after the module, a
duplicate field key, an unescaped quote in a value. The whole run exits
non-zero and prints nothing, because a partially-trusted rewrite of a log
file is worse than no rewrite at all.

```
$ loglint access.log
line 3: invalid timestamp '2026-09-16T10:23:03Z': timestamp must be exactly 24 characters (YYYY-MM-DDTHH:MM:SS.mmmZ), got 20
```

Pass `--lenient` when you know the input is messy and you want as much of
it recovered as possible instead of nothing:

```
$ loglint --lenient access.log
2026-09-16T10:23:01.123Z INFO  auth.session: user logged in | ip=10.0.0.5 user_id=42
2026-09-16T10:23:02.500Z WARN  cache.evict:  evicting stale entry | key=session:9f2c reason="ttl expired"
------------------------ ERROR db.pool:      connection refused
```

In lenient mode: an unparseable timestamp becomes a run of dashes instead
of aborting, an unrecognized level is kept verbatim instead of rejected, a
missing module colon falls back to using the raw token as the module name,
and a duplicate field key keeps the last occurrence instead of erroring.
Lines that are empty or all whitespace are always skipped, in both modes.

## Usage

```
loglint [--lenient] <file>
```

Exit code is `0` on a clean parse, `1` if any line failed to parse (in
strict mode nothing is printed in that case; in lenient mode the recovered
output is printed anyway before exiting non-zero), `2` on a usage error.

## Building

Standard library only, no external crates:

```
cargo build --release
```

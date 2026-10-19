# The king↔serf seam contract (FROZEN REFERENCE)

This is the one interface Eris must honor exactly. The urth king (vere C) and the
mars serf (Eris Rust) are **separate OS processes**; everything between them
crosses this pipe. Get this right and the two heritages compose; get it wrong and
nothing boots.

**Verified against** `vere` working tree `7f44b54f` and confirmed byte-identical
on the pin `origin/develop` `81404b1a`. Protocol version **2**. If an upstream
bump changes anything here, it is a release blocker — re-verify and re-freeze.

Source of truth in the submodule: `third_party/vere/pkg/vere/{newt.c,mars.c,lord.c}`
(mold comments at `mars.c:19-42` and `lord.c:11-47`).

## Process & transport

- The king spawns the serf as a child with `argv[1] = "work"` (run/resume) or
  `"boot"` (bootstrap). In stock vere this is a re-exec of `argv[0]`; **Eris
  redirects it to spawn the Rust serf binary** (the sole king-side patch).
- Three stdio pipes: **fd0** king→serf (writ), **fd1** serf→king (plea),
  **fd2** serf stderr (forwarded).
- **Framing** (`newt.c`): each message = 5-byte header + body.
  - `header[0]` = version tag `0x00` (reject otherwise).
  - `header[1..4]` = **little-endian u32 body length** (so a single message is
    capped at 4 GiB; reject len == 0).
  - `body` = the **`+jam` of a noun**. Send = `u3s_jam_xeno`; receive =
    `u3s_cue_xeno_with`.

## Writs (king → serf)

Serf parser: `mars.c:565-738`. King builders: `lord.c:608-643` + the
`u3_lord_*` entry points.

| writ    | jammed noun                                   | notes |
|---------|-----------------------------------------------|-------|
| `%poke` | `[%poke mil=@ ovum]`                           | `mil` = timeout ms; `ovum = [wire card]`. The event. |
| `%peek` | `[%peek mil=@ peek]`                           | `peek = [gang $%([%once ...] [%beam ...])]`; king sends `mil=0`, `[%peek [0 sample]]`. Scry/namespace read. |
| `%sync` | `[%sync %save ~]`                              | snapshot now. |
| `%live` | `[%live ?(%meld %pack) arg]`                   | compaction. |
| `%quiz` | `[%quiz ?(%quac %quic) ~]`                     | memory-usage query. |
| `%exit` | `[%exit ~]`                                    | shut down. |
| `%boot` | `[%boot com]`, `com = [pill=[p=@ q=(unit ovum)] vent=$%([%fake ship] [%dawn dawn-event]) more=(list prop)]` | only in the `boot` subprocess. |

## Pleas (serf → king)

Serf emitters: `_mars_gift` (`mars.c:275-294`), `_mars_fact` (`mars.c:237-271`).
King dispatcher: `lord.c:537-575`.

| plea    | jammed noun                                             | notes |
|---------|---------------------------------------------------------|-------|
| `%ripe` | `[%ripe [pro=%2 kel=wynn] [who=@p fake=?] eve=@ mug=@]` | **sent first, before anything.** Startup handshake. `kel`/`wynn` is the kelvin version list — must advertise a kelvin the king accepts (ties to the k138→k140 jet work). `mug` = `u3r_mug` of the kernel. |
| `%poke` | `[%poke p=(each (list ovum) (list goof))]`              | result of a `%poke` writ. head `&` → `[& effects]`; head `|` → `[| goofs]`. This is the old `%work` `%done`/`%bail`. |
| `%peek` | `[%peek p=(each (unit (cask)) goof)]`                   | scry result. |
| `%sync` | `[%sync eve=@ mug=@]`                                   | ack after a snapshot. |
| `%live` | `[%live ~]`                                             | ack of meld/pack. |
| `%slog` | `[%slog pri=@ tank]`                                    | debug/trace output (unsolicited). |
| `%flog` | `[%flog cord]`                                          | raw stderr line (unsolicited). |
| `%quiz` | `[%quiz ?(%quac %quic) p=*]`                            | memory report. |

## Ordering rules

- The king enforces a FIFO of outstanding writs (`lord.c:164-195`). Each
  **solicited** plea must match the head writ's type or the king bails.
- `%ripe`, `%slog`, `%flog` are **unsolicited** (no matching writ).
- `%ripe` must be the very first plea the serf emits.

## Minimum a conformant serf must implement

1. Read 5-byte-framed jammed writs from fd0.
2. Emit `%ripe` first with `pro=%2` and correct `kel`/`who`/`fake`/`eve`/`mug`.
3. Answer `%poke` with `[%poke (each (list ovum) (list goof))]`.
4. Answer `%peek`; honor `%sync %save`, `%live`, `%exit`.
5. Stream `%slog` (and `%flog`) for console/trace.
6. Reproduce vere's **mug** exactly (nockvm already does — see `PROVENANCE.md`),
   or run the king with `--soft-mugs` (`mars.c:998`) to disable the replay check.

## Things that are NOT in this contract (and must not leak across it)

- **The event log and replay.** In this vere the **serf owns the LMDB log and
  self-replays**; there is **no `%play` message**. The king (`pier.c`/`king.c`)
  has zero `u3_disk` references. The Eris serf owns its own log + replay
  (nockapp's `Serf` already does this with SQLite + PMA snapshots). The king
  only streams live pokes and trusts the serf to persist and recover.
- **The loom / snapshot format.** The serf owns its memory and snapshots itself;
  the king never reads the serf's loom. nockvm's 64-bit nouns / copy-based PMA
  are therefore invisible across this seam — they are a pure serf-internal
  concern. (Consequence: Eris cannot adopt a stock-vere ship's on-disk snapshot;
  migrate a ship by replaying its portable jammed event log instead.)

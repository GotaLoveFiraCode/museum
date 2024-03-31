# MUSEUM — Muse: Unleashing Music

You happen to have legally acquired a 9004-FLAC-files music library? You don’t
know what to do with it? The library is just to big to play random stuff, and
making your own playlists would take forever?

Well, you’re in luck! I happen to be working on a simple rust program that
catalogs your music in an SQLite database, and gives you intelligent music
queues!

Please help me.

## Roadmap

- [x] Actually play music…?
	+ [x] Controls: pause skip etc. Ideally through independent commands or TUI
		- [ ] Demonize.
		- [ ] TUI?
	+ [x] Play songs *from the database*, not just test songs…
- [x] Add install instructions.
- [ ] Better experience early on.
- [ ] Categorize stuff by genre/mood (separate music dirs using symlinks? Or SQLite DBs?)
- [x] Colors. Make stuff pretty.
- [ ] Decay system, so old data becomes irrelevant.
- [ ] Different file types without having to change the code…
- [ ] Figure out how to list `fd-find` as a dependency somehow. (Is this bullet point enough?)
- [x] Get a better acronym.
- [x] Modules. Use them. Not just one giant file. Please.
- [ ] Store more information with each song (metadata).
- [x] ~Use `anyhow` for proper error management.~
	+ [x] *Use `color-eyre` for proper error handling.*
- [x] Use clap crate for proper argument management.
- [x] Use SQLite DBs instead of JSON.

## How do I use this?

For now, you don’t. Feel free to contribute though!

Eventually, this will be a proper program that will play music *intelligently* (based on your preferences).

## Installing

`$ git clone …` \
`$ cargo install --path=museum`

### Dependencies: 

- `fd-find` \
	`$ cargo install fd-find`
- Patience \
	`$ echo "be patient"`
- Rust/cargo/etc. \
	`$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Alsa and a working audio system. Can’t play music without alsa…
	+ I’m not sure how this works on macOS…

### I don’t use FLAC? How do I pretend to use this?

Right now there’s a function called `find_music()`. This uses `fd-find` to go through your music directory, and find all FLAC files. You can go into the code (sorry), find that function (`$ rg 'find_music'` or `grep -r 'find_music' --exclude-dir=target --color=always`) and change the argument to fd-find:
```rust
Command::new("fd")
	.arg("-e")
	.arg("yourmusicfiletype")
	// …
```

Insert your preferred music file type (e.g. mp3) where it says *yourmusicfiletype* (keep the quotes). You can also add several different files types like this:
```rust
Command::new("fd")
	.arg("-e")
	.arg("flac")
	.arg("-e")
	.arg("mp3")
	// …
```

---

The following file types are supported:

- MP3
- WAV
- Vorbis
- FLAC (default)
- MP4 and ACC, but disabled by default. Open an issue if you want support for them.

Ignore the following content — most of it is outdated or irrelevant.

# PLANNING

## Create struct for 'song' (file)

include:
- absolute path,
- number of plays,
- number of skips,
- score

`score` is calculated with custom weights early on (under 30), and then
change to a logarithmic curve (1.2).

### Search

You should be able to search for specific file.

- Just search through SQL?

# Stuff I have enstablished

Each song saves two stats: touches and skips

`touches` are how often `museum` (the algo) has suggested the song.
  `skips` are how often the user has   skipped the song.

How often the user has actually listened to the whole song,
can be calculated as such: $listens = touches - skips$

# Things I want the algo to do

'boosted' means having a higher score.
Songs are rated with they’re score.
How do you calculate the score?

Feel free to add more stats that should be stored with each song (variables).

## Early on

Songs that have not been `touched`
very often — say, less than five times —
should be boosted, *even if* they have been
`skipped` a few times (e.g. `listens` is very low).

Example:

Score should be generous
$touches = 3$
$skips = 2$
$score = ???$

This is so the algo has the chance to get feedback on all
logged songs.
I.e. songs that have only been touched a few times,
are more likely to be suggested, so the algo can get an idea
of how much the user likes said song.

This means the algo doesn’t just end up *exclusively suggesting*
the first 50 songs it suggest.

Example:

Score should be ca. equally generous. \
$touches = 3$ \
$skips = 0$ \
$score = ???$ \

## Middle stage

When `touches` is still pretty low, `skips` shouldn’t take too much affect.
More emphasis should be put on how often the user listens to the whole song.

This way the user can skip a song a few times, without having to worry
about never seeing it again (snowball effect).

Example:

Score should be fairly generous, as the song *has* been listened to 30 times.
$$touches = 50
skips = 20
score = ???$$

Score should be very generous.
$$touches = 50
skips = 5
score = ???$$

Score should be strict.
$$touches = 50
skips = 45
score = ???$$

## Late stage

late-stage-songs: songs that have very high `touches`.

These songs should take skips very seriously,
so that if the user hasn’t enjoyed the song recently,
the skips take noticable effect.

Late stage songs should be downgraded (they’re score lowered)
very aggressively. Not much heed should be taken the the `touches` stat.

Example:

Score should be harsh
$$touches = 300
skips = 130
score = ???$$

Score should be generous
$$touches = 300
skips = 40
score = ???$$


## End result

The end result is that songs with low `touches`,
with medium `touches` and low `skips` (i.e. high `listens`),
with medium `touches` and medium `skips`,
and songs with high `touches` and low `skips` (i.e. high `listens`),
are suggested aggressively.

What are a few mathematical functions that matche all above data
as closely as possible. How do you further prevent snowballing?


# MUSEUM — Muse: Unleashing Music

A Curated Music Queue

## Roadmap

- [ ] Actually play music…?
- [x] Add install instructions.
- [ ] Better experience early on.
- [ ] Categorize stuff by genre/mood (separate SQLite DBs using symlinks?)
- [ ] Colors. Make stuff pretty.
- [ ] Decay system, so old data becomes irrelevant.
- [ ] Demonize.
- [ ] Different file types without having to change the code…
- [ ] Figure out how to list `fd-find` as a dependency somehow.
- [x] Get a better acronym.
- [ ] Modules. Use them. Not just one giant file. Please.
- [ ] Store more information with each song.
- [ ] TUI?
- [x] Use anyhow for proper error management. `urgent`
- [ ] Use clap crate for proper argument management.
- [x] Use SQLite DBs instead of JSON.

## How do I use this?

For now, you don’t. Feel free to contribute though!

Eventually, this will be a proper program that will play music *intelligently* (based on your preferences).

## Installing

`$ git clone …`
`$ cargo install --path=museum`

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
Command::new("fd)
	.arg("-e")
	.arg("flac")
	.arg("-e")
	.arg("mp3")
	// …
```

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


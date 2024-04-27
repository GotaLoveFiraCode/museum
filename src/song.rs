/// A collection of songs that belong together.
/// What songs belong together, etc., is algorithmically learned from the user.
/// This can be though of, as the user’s _mood_.
/// WIP
struct Mood {
    pub songs: Vec<Song>,
    pub id: u8,
}

impl Mood {
    /// Taking the previous songs, which existing `Mood` is this most
    /// similar to? Considering that, which song from the found `Mood`s
    /// has the highest score?
    ///
    /// This function essentially tries to predict, given the list of of
    /// songs the user has already listened to (fully, so has not skipped),
    /// which song they will enjoy next. It essentially gueses the user’s
    /// mood. It takes previous songs, but also score, into account.
    pub fn predict(prev_songs: Vec<Song>) -> color_eyre::Result<Song> {
        todo!("Will work on slowly.");
    }
}

/// How a song---and associated data---is stored in the database.
#[derive(Debug, Default, Clone)]
pub struct Song {
    /// SQL — limit on how many songs can be cataloged.
    /// Usefull for removing duplicates later.
    #[allow(dead_code)]
    pub id: u32,
    // Change to PathBuf for stability?
    pub path: String,
    /// How often the song has been included in the que.
    /// $listens = touches -skips$
    /// $score   = listens - skips * aggression$
    pub touches: u32,
    /// When the user skips the song.
    pub skips: u32,
    /// If a song is `loved`, it is very nearly always prefered
    /// when a choice is being made by the algo between songs.
    /// Simplified: it doubles the score.
    pub loved: Love,
    /// Calculated score.
    pub score: Option<f64>,
}

#[derive(Default, Debug, Clone)]
pub enum Love {
    #[default]
    False = 1,
    True = 2,
}

impl Song {
    #![allow(dead_code)]
    #[must_use]
    /// Calculates the "score" of the song in question.
    /// Takes touches and skips into account.
    /// Dynamically changes weights based on touches.
    pub fn calc_score(&self) -> f64 {
        let love = f64::from(self.loved.clone() as u8);
        let listens = f64::from(self.touches - self.skips) * love;
        let skips = f64::from(self.skips);
        let mut score: f64;

        // 30 seems good, as the difference
        // first gets doubled (5 -> 10),
        // and then 10 -> 15,
        // and finally doubled again (15 -> 30).
        score = if self.touches < 30 {
            let (weight_listens, weight_skips) = self.weight();
            weight_listens * listens - weight_skips * skips
        } else {
            // Skips may be larger than listens.
            self.dampen() * listens - self.dampen() * skips
        };

        if score < 0.0 {
            score = 0.0;
        }

        log::trace!("Calculated `{}' score for `{}' song.", score, self.path);
        score
    }

    /// Weight calculation for songs with low touches (<30).
    /// Returns (`listens_weight`, `skips_weight`)
    ///
    /// Could, in theory, be used with values over 30,
    /// but this is not recommended — use [logarithmic
    /// dampening](<`fn dampen(touches)`>) instead.
    ///
    /// # `touches < 5`
    ///
    /// Listens are more important than skips
    /// This means that early, anecdotal skips are disregarded.
    ///
    /// # `touches <= 15`
    ///
    /// Listens are equally important to skips.
    ///
    /// # `touches > 15`
    ///
    /// Skips are more important than listens.
    /// this means skips still take an effect,
    /// and the algo learns with stability.
    fn weight(&self) -> (f64, f64) {
        // Need fine-tuning.
        let low = 0.5;
        let medium = 1.0;
        let high = 2.0;

        // These could also use some fine-tuning.
        // Currently using this *with* a logarithmic function
        // for the later stages. Thats why `big_threshold`
        // is so small.
        let small_threshold = 5;
        let big_threshold = 15;

        if self.touches < small_threshold {
            // Listens are more important than skips
            // This means that early, anecdotal skips are disregarded.
            (high, low)
        } else if self.touches <= big_threshold {
            // Listens are equally important to skips.
            (medium, medium)
        } else {
            // Skips are more important than listens.
            // So skips still take an effect,
            // and the algo learns with stability.
            (low, high)
        }
    }

    /// Logarithmic dampening function.
    /// Returns weight.
    ///
    /// Meant to be used for songs with over 30 touches.
    /// Very slow increase in weight, as touches incease,
    /// meaning that skips steadily have more importance.
    ///
    /// Causes recent preferences to rule king.
    ///
    fn dampen(&self) -> f64 {
        // `+1` just in case.
        // `1.2` seems to be ideal.
        let weight = f64::from(self.touches + 1).log(1.2);
        log::trace!("Calculated logarithmic weight `{}'.", weight);
        weight
    }
}

package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.Season
import com.fantatt.fantattbackend.db.repos.SeasonRepository
import org.springframework.stereotype.Component
import java.time.LocalDate

// first of october
val SEASON_START: LocalDate = LocalDate.of(0, 10, 1)
// end of june
val SEASON_END: LocalDate = LocalDate.of(0, 7, 1)

/**
 * The season year is the year when the season begun.
 * The current season is the one that must be played
 */
@Component
class SeasonManager(
    val seasonRepository: SeasonRepository
) {
    fun getCurrent(): Season {
        val currentSeasonYear = getCurrentSeasonYear()
        val out = seasonRepository.findByYear(getCurrentSeasonYear())
        return out.orElseGet { seasonRepository.save(Season(currentSeasonYear)) }
    }

    fun getCurrentSeasonYear(): Int {
        val currentDate = LocalDate.now()
        // if the date is after the season end of the year, the year season is in progress
        return if (currentDate > SEASON_END.withYear(currentDate.year))
            currentDate.year
            else currentDate.year -1
    }
}
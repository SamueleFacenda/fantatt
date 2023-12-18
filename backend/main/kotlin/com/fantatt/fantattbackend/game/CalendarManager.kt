package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.Round
import com.fantatt.fantattbackend.db.entities.Season
import com.fantatt.fantattbackend.db.repos.SeasonRepository
import org.springframework.stereotype.Component
import java.sql.Date
import java.time.LocalDate

// first of october
val SEASON_START: LocalDate = LocalDate.of(0, 10, 1)
// end of june
val SEASON_END: LocalDate = LocalDate.of(1, 7, 1)
// Christmas break
val SEASON_PAUSE: LocalDate = LocalDate.of(0, 12, 25)
val SEASON_RESUME: LocalDate = LocalDate.of(1, 1, 10)

/**
 * The season year is the year when the season begun.
 * The current season is the one that must be played
 */
@Component
class CalendarManager(
    val seasonRepository: SeasonRepository
) {
    fun getCurrentSeason(): Season {
        val currentSeasonYear = getCurrentSeasonYear()
        val out = seasonRepository.findByYear(getCurrentSeasonYear())
        return out ?: createSeason(currentSeasonYear)
    }

    fun getCurrentSeasonYear(): Int {
        val currentDate = LocalDate.now()
        // if the date is after the season end of the year, the year season is in progress
        return if (currentDate > SEASON_END.withYear(currentDate.year))
            currentDate.year
            else currentDate.year -1
    }

    fun createSeason(year: Int): Season {
        val season = Season(year)
        seasonRepository.save(season)
        generateRounds(season)
        return season
    }

    fun generateRounds(season: Season) {
        TODO()
    }

    fun getCurrentRoundNum(): Int {
        val rounds = getCurrentSeason().rounds
        val currentDate = Date.valueOf(LocalDate.now())
        return rounds.filter {
            it.startDate <= currentDate
        }.maxByOrNull(Round::startDate)?.index ?: throw IllegalStateException("No round found, should be next season")
    }
}
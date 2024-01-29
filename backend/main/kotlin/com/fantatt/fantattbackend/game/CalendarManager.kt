package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.Round
import com.fantatt.fantattbackend.db.entities.Season
import com.fantatt.fantattbackend.db.repos.RoundRepository
import com.fantatt.fantattbackend.db.repos.SeasonRepository
import org.springframework.stereotype.Component
import java.time.LocalDate
import java.time.DayOfWeek.*
import java.time.Duration
import java.time.LocalDateTime
import java.time.Period
import java.time.temporal.TemporalAdjusters.nextOrSame

// first of october
val SEASON_START: LocalDate = LocalDate.of(0, 10, 1)
// end of june
val SEASON_END: LocalDate = LocalDate.of(1, 7, 1)
// Christmas break
val SEASON_PAUSE: LocalDate = LocalDate.of(0, 12, 25)
val SEASON_RESUME: LocalDate = LocalDate.of(1, 1, 10)

// to add to the round date (sunday at 00:00), to get the start and end of the round
val ROUND_START_OFFSET: Duration = Duration.ofHours(-12)
val ROUND_END_OFFSET: Duration = Duration.ofHours(20)

/**
 * The season year is the year when the season begun.
 * The current season is the one that must be played
 */
@Component
class CalendarManager(
    val seasonRepository: SeasonRepository,
    val roundRepository: RoundRepository
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
        val rounds = generateRounds(season)
        roundRepository.saveAll(rounds)
        return season
    }

    fun generateRounds(season: Season): List<Round> {
        val sundays = getRoundDatesForYear(season.year.toLong())
        return sundays
            .map { it.atStartOfDay() }
            .mapIndexed { index, date ->
                Round(
                    index = index + 1,
                    startTime = date.plus(ROUND_START_OFFSET),
                    endTime = date.plus(ROUND_END_OFFSET),
                    season = season
                )
            }
    }

    private fun getRoundDatesForYear(year: Long): List<LocalDate> {
        val beforeChristmas = generateSundaysInInterval(
            start = SEASON_START.plusYears(year),
            end = SEASON_PAUSE.plusYears(year)
        )
        val afterChristmas = generateSundaysInInterval(
            start = SEASON_RESUME.plusYears(year),
            end = SEASON_END.plusYears(year)
        )
        return beforeChristmas + afterChristmas
    }

    fun generateSundaysInInterval(start: LocalDate, end: LocalDate): List<LocalDate> {
        return start
            .with(nextOrSame(SUNDAY))
            .datesUntil(end, Period.ofDays(7))
            .toList()
    }

    fun getCurrentRoundNum(): Int {
        val rounds = getCurrentSeason().rounds
        val currentTime = LocalDateTime.now()
        return rounds.filter {
            it.startTime <= currentTime
        }.maxByOrNull(Round::startTime)?.index ?: throw IllegalStateException("No round found, should be next season")
    }
}
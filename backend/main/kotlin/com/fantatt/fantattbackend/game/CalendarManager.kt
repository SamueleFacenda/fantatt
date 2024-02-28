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
val ROUND_RESULT_OFFSET: Duration = Duration.ofDays(2).plusHours(12)

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
                    index = index,
                    startTime = date.plus(ROUND_START_OFFSET),
                    endTime = date.plus(ROUND_END_OFFSET),
                    resultTime = date.plus(ROUND_RESULT_OFFSET),
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

    fun getPreviousRound(round: Round): Round {
        if (round.index == 0)
            throw IllegalStateException("No previous round")

        return roundRepository.findByIndexAndSeason(round.index - 1, round.season) ?: throw IllegalStateException("No previous round")
    }

    fun isRoundInProgress(): Boolean {
        // TODO cache
        val current = LocalDateTime.now()
        return roundRepository.findByStartTimeAfterAndEndTimeBefore(current, current) != null
    }

    fun isResultWaitTime(): Boolean {
        // TODO cache
        val current = LocalDateTime.now()
        return roundRepository.findByEndTimeAfterAndResultTimeBefore(current, current) != null
    }

    fun getNextStartingRound(): Round? {
        // TODO cache
        val current = LocalDateTime.now()
        return roundRepository.findTopByStartTimeAfterOrderByStartTime(current)
    }

    /**
     * get the round not yet started (assumes that the rounds
     * from the next season are not in the database)
     */
    fun getRemainingRounds(): List<Round> {
        // TODO cache
        val current = LocalDateTime.now()
        return roundRepository.findAllByStartTimeAfterOrderByIndex(current)
    }

}
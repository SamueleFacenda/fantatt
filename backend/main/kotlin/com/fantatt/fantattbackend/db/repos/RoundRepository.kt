package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.Round
import com.fantatt.fantattbackend.db.entities.Season
import org.springframework.data.repository.CrudRepository
import java.sql.Date
import java.time.LocalDateTime

interface RoundRepository: CrudRepository<Round, Long> {
    fun findByIndexAndSeason(index: Int, season: Season): Round?
    fun findByStartTimeAfterAndEndTimeBefore(startTime: LocalDateTime, endTime: LocalDateTime): Round?
    fun findByEndTimeAfterAndResultTimeBefore(endTime: LocalDateTime, resultTime: LocalDateTime): Round?
    fun findTopByStartTimeAfterOrderByStartTime(startTime: LocalDateTime): Round?
    fun findAllByStartTimeAfterOrderByIndex(startTime: LocalDateTime): List<Round>
}
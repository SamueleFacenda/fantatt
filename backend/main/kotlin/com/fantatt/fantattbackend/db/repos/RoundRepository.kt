package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.Round
import com.fantatt.fantattbackend.db.entities.Season
import org.springframework.data.repository.CrudRepository
import java.sql.Date
import java.time.LocalDateTime

interface RoundRepository: CrudRepository<Round, Long> {
    fun findByIndex(index: Int): Round?
    fun findByIndexAndSeason(index: Int, season: Season): Round?
    fun findTopBySeasonYearAndStartTimeBeforeOrderByIndex(year: Int, startTime: LocalDateTime): Round?
}
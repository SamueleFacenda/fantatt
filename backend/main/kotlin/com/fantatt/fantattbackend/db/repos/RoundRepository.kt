package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.RoundEntity
import com.fantatt.fantattbackend.db.entities.SeasonEntity
import org.springframework.data.repository.CrudRepository
import java.time.LocalDateTime

interface RoundRepository: CrudRepository<RoundEntity, Long> {
    fun findByIndexAndSeason(index: Int, season: SeasonEntity): RoundEntity?
    fun findByStartTimeAfterAndEndTimeBefore(startTime: LocalDateTime, endTime: LocalDateTime): RoundEntity?
    fun findByEndTimeAfterAndResultTimeBefore(endTime: LocalDateTime, resultTime: LocalDateTime): RoundEntity?
    fun findTopByStartTimeAfterOrderByStartTime(startTime: LocalDateTime): RoundEntity?
    fun findAllByStartTimeAfterOrderByIndex(startTime: LocalDateTime): List<RoundEntity>
}
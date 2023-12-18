package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.Round
import com.fantatt.fantattbackend.db.entities.Season
import org.springframework.data.repository.CrudRepository

interface RoundRepository: CrudRepository<Round, Long> {
    fun findByIndex(index: Int): Round?
    fun findByIndexAndSeason(index: Int, season: Season): Round?
}
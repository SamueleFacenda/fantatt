package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.Season
import org.springframework.data.repository.CrudRepository

interface SeasonRepository: CrudRepository<Season, Long> {
    fun existsByYear(year: Int): Boolean
    fun findByYear(year :Int): Season?
}
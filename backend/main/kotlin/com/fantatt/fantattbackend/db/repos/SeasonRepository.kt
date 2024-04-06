package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.SeasonEntity
import org.springframework.data.repository.CrudRepository

interface SeasonRepository: CrudRepository<SeasonEntity, Long> {
    fun existsByYear(year: Int): Boolean
    fun findByYear(year :Int): SeasonEntity?
}
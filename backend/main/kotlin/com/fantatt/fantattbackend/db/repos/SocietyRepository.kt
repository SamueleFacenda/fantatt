package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.Society
import org.springframework.data.repository.CrudRepository

interface SocietyRepository: CrudRepository<Society, Long> {
    fun findAllByLeagueIdOrderByPointsDesc(leagueId: Long): List<Society>
}
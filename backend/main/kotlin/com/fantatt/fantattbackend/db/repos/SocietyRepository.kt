package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.SocietyEntity
import org.springframework.data.repository.CrudRepository

interface SocietyRepository: CrudRepository<SocietyEntity, Long> {
    fun findAllByLeagueIdOrderByPointsDesc(leagueId: Long): List<SocietyEntity>
}
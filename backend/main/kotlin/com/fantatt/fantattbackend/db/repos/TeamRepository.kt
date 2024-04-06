package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.LeagueEntity
import com.fantatt.fantattbackend.db.entities.SocietyEntity
import com.fantatt.fantattbackend.db.entities.TeamEntity
import org.springframework.data.jpa.repository.Query
import org.springframework.data.repository.CrudRepository

interface TeamRepository: CrudRepository<TeamEntity, Long> {
    @Query("SELECT t FROM TeamEntity t WHERE t.society.league = :league AND t.division = :division ORDER BY t.score DESC")
    fun findAllByLeagueAndDivisionOrderByScoreDesc(league: LeagueEntity, division: Int): List<TeamEntity>
    fun findBySocietyIdAndName(societyId: Long, name: String): TeamEntity?
    fun findAllBySociety(society: SocietyEntity): List<TeamEntity>
}
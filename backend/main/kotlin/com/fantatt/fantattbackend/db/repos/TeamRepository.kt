package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.League
import com.fantatt.fantattbackend.db.entities.Society
import com.fantatt.fantattbackend.db.entities.Team
import org.springframework.data.jpa.repository.Query
import org.springframework.data.repository.CrudRepository

interface TeamRepository: CrudRepository<Team, Long> {
    @Query("SELECT t FROM Team t WHERE t.society.league = :league AND t.division = :division ORDER BY t.score DESC")
    fun findAllByLeagueAndDivisionOrderByScoreDesc(league: League, division: Int): List<Team>
    fun findBySocietyAndDivision(society: Society, division: Int): Team?
}
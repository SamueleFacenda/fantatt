package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.Participation
import com.fantatt.fantattbackend.db.entities.ParticipationId
import com.fantatt.fantattbackend.db.entities.Round
import com.fantatt.fantattbackend.db.entities.Team
import org.springframework.data.jpa.repository.Query
import org.springframework.data.repository.CrudRepository

interface ParticipationRepository: CrudRepository<Participation, ParticipationId> {
    fun findAllByTeamAndRound(team: Team, round: Round): List<Participation>

    @Query("SELECT p FROM Participation p WHERE p.team.id = :teamId AND p.round.index = :roundIndex")
    fun findAllByTeamAndRound(teamId: Long, roundIndex: Int): List<Participation>
}
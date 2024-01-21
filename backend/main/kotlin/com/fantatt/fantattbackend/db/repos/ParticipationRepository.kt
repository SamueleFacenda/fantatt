package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.Participation
import com.fantatt.fantattbackend.db.entities.ParticipationId
import com.fantatt.fantattbackend.db.entities.Round
import com.fantatt.fantattbackend.db.entities.Society
import org.springframework.data.jpa.repository.Query
import org.springframework.data.repository.CrudRepository

interface ParticipationRepository: CrudRepository<Participation, ParticipationId> {
    fun findAllByTeamAndRound(team: Society, round: Round): List<Participation>

    @Query("SELECT p FROM Participation p WHERE p.team = :team AND p.round.index = :roundIndex")
    fun findAllByTeamAndRound(team: Society, roundIndex: Int): List<Participation>
}
package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.*
import org.springframework.data.jpa.repository.Query
import org.springframework.data.repository.CrudRepository

interface ParticipationRepository: CrudRepository<Participation, ParticipationId> {
    fun findAllByTeamAndRound(team: Team, round: Round): List<Participation>

}
package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.*
import org.springframework.data.repository.CrudRepository

interface ParticipationRepository: CrudRepository<ParticipationEntity, ParticipationId> {
    fun findAllByTeamAndRound(team: TeamEntity, round: RoundEntity): List<ParticipationEntity>

}
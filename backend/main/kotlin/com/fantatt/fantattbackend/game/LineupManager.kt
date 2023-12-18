package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.Participation
import com.fantatt.fantattbackend.db.repos.ParticipationRepository
import org.springframework.stereotype.Component

@Component
class LineupManager(
    val participationRepository: ParticipationRepository,
    val roundManager: RoundManager
) {

    fun getLineup(teamId: Long,
                  roundNum: Int = roundManager.getCurrentRound()
    ): List<Participation> {
        // recursion base case
        if (roundNum < 0)
            return createDefaultLineup(teamId)

        val current = participationRepository.findAllByTeamAndRound(teamId, roundNum)

        if (current.isEmpty())
            return getLineup(teamId, roundNum - 1)

        return current
    }

    fun createDefaultLineup(teamId: Long): List<Participation> {
        TODO()
    }
}
package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.Participation
import com.fantatt.fantattbackend.db.entities.Team
import com.fantatt.fantattbackend.db.repos.ParticipationRepository
import com.fantatt.fantattbackend.db.repos.RoundRepository
import com.fantatt.fantattbackend.db.repos.TeamRepository
import org.springframework.stereotype.Component

const val STARTING_LINEUP_SIZE = 3
const val BENCH_SIZE = 5

@Component
class LineupManager(
    val participationRepository: ParticipationRepository,
    val calendarManager: CalendarManager,
    val roundRepository: RoundRepository
) {

    fun getLineup(team: Team,
                  roundNum: Int = calendarManager.getCurrentRound()
    ): List<Participation> {
        // recursion base case
        if (roundNum < 0)
            return createDefaultLineup(team, roundNum)

        val current = participationRepository.findAllByTeamAndRound(team, roundNum)

        if (current.isEmpty())
            return createLineupFromPreviousRound(team, roundNum)

        return current
    }

    fun createLineupFromPreviousRound(team: Team, roundNum: Int): List<Participation> {
        val prev = getLineup(team, roundNum - 1)
        val out = prev.map {
            it.copy(round = roundRepository.findByIndexAndSeason(roundNum, team.league.season)?:
                throw Exception("Round $roundNum not found"))
        }
        return participationRepository.saveAll(out).toList()
    }

    fun createDefaultLineup(team: Team, roundNum: Int): List<Participation> {
        val players = team.players.take(STARTING_LINEUP_SIZE + BENCH_SIZE).sortedBy { -it.points }
        val round = calendarManager.getRound(roundNum, team.league.season)
        val out = players.mapIndexed {
            index, player -> Participation(
                player = player,
                team = team,
                round = round,
                order = index
            )
        }
        return participationRepository.saveAll(out).toList()
    }
}
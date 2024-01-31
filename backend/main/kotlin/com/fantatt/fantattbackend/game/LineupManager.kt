package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.Participation
import com.fantatt.fantattbackend.db.entities.Player
import com.fantatt.fantattbackend.db.entities.Society
import com.fantatt.fantattbackend.db.entities.Team
import com.fantatt.fantattbackend.db.repos.ParticipationRepository
import com.fantatt.fantattbackend.db.repos.PlayerRepository
import com.fantatt.fantattbackend.db.repos.RoundRepository
import org.springframework.stereotype.Component

const val STARTING_LINEUP_SIZE = 3
const val BENCH_SIZE = 2

/**
 * This is a wrapper around the ParticipationRepository that handles the logic of creating a lineup.
 * With this the lineup creation is lazy, and the lineup is created only when needed.
 * To make so, when needed, a lineup for all the society's teams is created.
 */
@Component
class LineupManager(
    val participationRepository: ParticipationRepository,
    val calendarManager: CalendarManager,
    val roundRepository: RoundRepository,
) {

    fun getLineup(
        team: Team,
        roundNum: Int = calendarManager.getCurrentRoundNum()
    ): List<Participation> {

        val current = participationRepository.findAllByTeamAndRoundIndex(team, roundNum)

        if (current.isEmpty() && roundNum == 0)
            return createDefaultLineup(team.society, roundNum)[team]!!

        if (current.isEmpty())
            return createLineupFromPreviousRound(team.society, roundNum)[team]!!

        return current
    }

    private fun createLineupFromPreviousRound(society: Society, roundNum: Int): Map<Team, List<Participation>> {
        val round = roundRepository.findByIndexAndSeason(roundNum, society.league.season)?: throw IllegalStateException("Round $roundNum not found")
        // for every team get the previous lineup, map the round to the current one and save it
        return society.teams
            .associateWith { getLineup(it, roundNum - 1) }
            .mapValues { pair -> pair.value.map { it.copy(round = round) } }
            .mapValues { pair -> participationRepository.saveAll(pair.value).toList() }
    }

    /**
     * Creates a default lineup for the given society.
     * The logic is the following:
     * take the best players sorted and make n teams (n = number of teams in the society)
     * take the remaining players and make n benches
     * merge these lists to have the lineups lists.
     * Then create the participation objects for each team
     */
    private fun createDefaultLineup(society: Society, roundNum: Int): Map<Team, List<Participation>> {
        val round = roundRepository.findByIndexAndSeason(roundNum, society.league.season)?: throw IllegalStateException("Round $roundNum not found")
        val teams = society.teams.sortedBy { it.division }
        val lineups = makeLineupsForNTeams(teams.size, society.players)
        return teams
            .zip(lineups)
            .toMap()
            .mapValues { pair ->
                pair.value.mapIndexed { order, player ->
                    Participation(
                        player = player,
                        team = pair.key,
                        round = round,
                        order = order
                    )
                }
            }
            .mapValues { pair -> participationRepository.saveAll(pair.value).toList() }
    }

    private fun makeLineupsForNTeams(n: Int, players: List<Player>): List<List<Player>> {
        val sortedPlayers = players.sortedBy { -it.points }

        val startingPlayers = sortedPlayers
            .chunked(STARTING_LINEUP_SIZE)
            .take(n)
        val benchPlayers = sortedPlayers
            .drop(STARTING_LINEUP_SIZE * n)
            .chunked(BENCH_SIZE)
            .take(n)

        return startingPlayers
            .zip(benchPlayers)
            .map { it.first + it.second } // starting plus bench sorted
    }
}
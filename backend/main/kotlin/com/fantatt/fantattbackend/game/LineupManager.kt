package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.*
import com.fantatt.fantattbackend.db.repos.ParticipationRepository
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
) {

    fun getLineup(
        team: Team,
        round: Round
    ): List<Participation> {

        val current = participationRepository.findAllByTeamAndRound(team, round)

        if (current.isEmpty() && round.index == 0)
            return createDefaultLineup(team.society, round)[team]!!

        if (current.isEmpty())
            return createLineupFromPreviousRound(team.society, round)[team]!!

        return current
    }

    private fun createLineupFromPreviousRound(society: Society, round: Round): Map<Team, List<Participation>> {
        val prevRound = calendarManager.getPreviousRound(round)
        // for every team get the previous lineup, map the round to the current one and save it
        return society.teams
            .associateWith { getLineup(it, prevRound) }
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
    private fun createDefaultLineup(society: Society, round: Round): Map<Team, List<Participation>> {
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
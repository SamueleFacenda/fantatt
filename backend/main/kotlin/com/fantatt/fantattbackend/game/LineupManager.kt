package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.Participation
import com.fantatt.fantattbackend.db.entities.Society
import com.fantatt.fantattbackend.db.repos.ParticipationRepository
import com.fantatt.fantattbackend.db.repos.RoundRepository
import org.springframework.stereotype.Component

const val STARTING_LINEUP_SIZE = 3
const val BENCH_SIZE = 5

@Component
class LineupManager(
    val participationRepository: ParticipationRepository,
    val calendarManager: CalendarManager,
    val roundRepository: RoundRepository
) {

    fun getLineup(society: Society,
                  roundNum: Int = calendarManager.getCurrentRoundNum()
    ): List<Participation> {
        // recursion base case
        if (roundNum < 0)
            return createDefaultLineup(society, roundNum)

        val current = participationRepository.findAllByTeamAndRound(society, roundNum)

        if (current.isEmpty())
            return createLineupFromPreviousRound(society, roundNum)

        return current
    }

    fun createLineupFromPreviousRound(society: Society, roundNum: Int): List<Participation> {
        val prev = getLineup(society, roundNum - 1)
        val out = prev.map {
            it.copy(round = roundRepository.findByIndexAndSeason(roundNum, society.league.season) ?:
                throw IllegalStateException("Round $roundNum not found") ) }

        return participationRepository.saveAll(out).toList()
    }

    fun createDefaultLineup(society: Society, roundNum: Int): List<Participation> {
        val players = society.players.take(STARTING_LINEUP_SIZE + BENCH_SIZE).sortedBy { -it.points }
        val round = roundRepository.findByIndexAndSeason(roundNum, society.league.season)?: throw IllegalStateException("Round $roundNum not found")
        val out = players.mapIndexed {
            index, player -> Participation(
                player = player,
                team = society,
                round = round,
                order = index
            )
        }
        return participationRepository.saveAll(out).toList()
    }
}
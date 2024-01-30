package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.Player
import com.fantatt.fantattbackend.db.entities.Society
import org.springframework.data.jpa.repository.Query
import org.springframework.data.repository.CrudRepository


interface PlayerRepository: CrudRepository<Player, Long> {

    @Query("SELECT p FROM Player p WHERE :society MEMBER OF p.societies AND p NOT IN (SELECT p FROM Player JOIN Participation pt WHERE pt.round.index = :roundIndex AND pt.team.society = :society)")
    fun findAllBySocietyAndNotParticipatingInRound(society: Society, roundIndex: Int): List<Player>
}
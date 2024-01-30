package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.Player
import com.fantatt.fantattbackend.db.entities.Society
import org.springframework.data.jpa.repository.Query
import org.springframework.data.repository.CrudRepository


interface PlayerRepository: CrudRepository<Player, Long> {
}
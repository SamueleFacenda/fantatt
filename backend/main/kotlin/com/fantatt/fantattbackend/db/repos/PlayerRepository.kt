package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.Player
import org.springframework.data.repository.CrudRepository


interface PlayerRepository: CrudRepository<Player, Long> {
}
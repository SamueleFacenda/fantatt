package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.PlayerEntity
import org.springframework.data.repository.CrudRepository


interface PlayerRepository: CrudRepository<PlayerEntity, Long> {
}
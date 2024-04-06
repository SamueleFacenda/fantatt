package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.UserEntity
import org.springframework.data.repository.CrudRepository

interface UserRepository: CrudRepository<UserEntity, Long> {
    fun findByUsername(username: String): UserEntity?
    fun existsByUsername(username: String): Boolean
    fun existsByEmailOrUsername(email: String, username: String): Boolean
}
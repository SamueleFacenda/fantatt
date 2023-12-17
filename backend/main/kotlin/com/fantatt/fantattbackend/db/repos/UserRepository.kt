package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.User
import org.springframework.data.repository.CrudRepository
import java.util.Optional

interface UserRepository: CrudRepository<User, Long> {
    fun findByUsername(username: String): User?
    fun existsByUsername(username: String): Boolean
    fun existsByEmailOrUsername(email: String, username: String): Boolean
}
package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.User
import org.springframework.data.repository.CrudRepository
import org.springframework.data.rest.core.annotation.RestResource

@RestResource(exported = false)
interface UserRepository: CrudRepository<User, Long> {
    fun findByUsername(username: String): User
    fun existsByUsername(username: String): Boolean
    fun existsByEmailOrUsername(email: String, username: String): Boolean

}
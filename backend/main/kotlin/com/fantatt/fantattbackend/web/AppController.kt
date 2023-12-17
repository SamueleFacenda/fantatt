package com.fantatt.fantattbackend.web

import com.fantatt.fantattbackend.db.entities.User
import com.fantatt.fantattbackend.db.repos.UserRepository
import org.springframework.web.bind.annotation.PostMapping
import org.springframework.web.bind.annotation.RequestBody
import org.springframework.web.bind.annotation.RestController

@RestController
class AppController(
    val userRepository: UserRepository
) {
    @PostMapping("/register")
    fun register(@RequestBody newUser: User): User {
        if (userRepository.existsByEmailOrUsername(newUser.email, newUser.username)) {
            throw Exception("User already exists")
        }
        return userRepository.save(newUser)
    }
}
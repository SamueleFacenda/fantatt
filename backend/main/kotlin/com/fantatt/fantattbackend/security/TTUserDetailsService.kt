package com.fantatt.fantattbackend.security

import com.fantatt.fantattbackend.db.repos.UserRepository
import org.springframework.security.core.GrantedAuthority
import org.springframework.security.core.userdetails.User
import org.springframework.security.core.userdetails.UserDetails
import org.springframework.security.core.userdetails.UserDetailsService
import org.springframework.security.core.userdetails.UsernameNotFoundException
import org.springframework.stereotype.Service

@Service
class TTUserDetailsService(
    val userRepository: UserRepository
): UserDetailsService {

    override fun loadUserByUsername(username: String): UserDetails {
        val user = userRepository.findByUsername(username)?: throw UsernameNotFoundException(username)
        return User(user.username, user.password, listOf(GrantedAuthority { "ROLE_USER" }))
    }
}
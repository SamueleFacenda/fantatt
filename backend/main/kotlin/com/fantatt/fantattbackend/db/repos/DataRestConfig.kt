package com.fantatt.fantattbackend.db.repos

import org.springframework.context.annotation.Bean
import org.springframework.context.annotation.Configuration
import org.springframework.data.rest.core.mapping.RepositoryDetectionStrategy.RepositoryDetectionStrategies.ANNOTATED

@Configuration
class DataRestConfig {

    @Bean
    fun repositoryDetectionStrategy() = ANNOTATED
}
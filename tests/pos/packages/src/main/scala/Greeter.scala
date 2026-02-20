package com.example.service

import com.example.model.Person

object Greeter:
  def greet(p: Person): String =
    s"Hello, ${p.greetingName}! You are ${p.age} years old."

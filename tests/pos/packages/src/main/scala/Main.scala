package com.example

import com.example.model.Person
import com.example.service.Greeter

@main def packagesMain(): Unit =
  val people = List(Person("Alice", 30), Person("Bob", 12))
  people.foreach(p => println(Greeter.greet(p)))

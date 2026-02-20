package com.example.model

case class Person(name: String, age: Int):
  def greetingName: String = if age < 18 then name else s"Mr/Ms $name"

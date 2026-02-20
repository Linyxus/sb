import upickle.default.*

case class Address(street: String, city: String) derives ReadWriter
case class Person(name: String, age: Int, address: Address) derives ReadWriter

@main def ujsonMain(): Unit =
  val alice = Person("Alice", 30, Address("123 Main St", "Springfield"))
  val json = write(alice, indent = 2)
  println(s"Serialized:\n$json")

  val parsed = read[Person](json)
  println(s"Deserialized: $parsed")
  assert(parsed == alice)

  // Work with ujson AST
  val obj = ujson.Obj(
    "items" -> ujson.Arr(
      ujson.Obj("id" -> 1, "label" -> "foo"),
      ujson.Obj("id" -> 2, "label" -> "bar")
    )
  )
  val labels = obj("items").arr.map(_("label").str).toList
  println(s"Labels: $labels")

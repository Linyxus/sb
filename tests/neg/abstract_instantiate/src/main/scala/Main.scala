abstract class Animal(name: String):
  def speak(): String

@main def app(): Unit =
  val a = new Animal("dog")  // can't instantiate abstract
  println(a.speak())

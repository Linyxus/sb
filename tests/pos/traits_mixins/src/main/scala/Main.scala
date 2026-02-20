sealed trait Animal:
  def name: String
  def sound: String
  def describe: String = s"$name says $sound"

trait Domestic:
  self: Animal =>
  def owner: String
  def describeOwned: String = s"$name is owned by $owner"

trait Trainable:
  self: Animal =>
  def tricks: List[String]
  def showTricks: String = s"$name can: ${tricks.mkString(", ")}"

case class Dog(name: String, owner: String, tricks: List[String])
    extends Animal with Domestic with Trainable:
  val sound = "Woof"

case class Cat(name: String, owner: String)
    extends Animal with Domestic:
  val sound = "Meow"

case class Wolf(name: String) extends Animal:
  val sound = "Howl"

@main def traitsMain(): Unit =
  val animals: List[Animal] = List(
    Dog("Rex", "Alice", List("sit", "shake", "roll over")),
    Cat("Whiskers", "Bob"),
    Wolf("Grey")
  )
  animals.foreach(a => println(a.describe))
  animals.collect { case d: Domestic => d }.foreach(d => println(d.describeOwned))
  animals.collect { case t: Trainable => t }.foreach(t => println(t.showTricks))

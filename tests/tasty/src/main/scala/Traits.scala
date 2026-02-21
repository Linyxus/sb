// Traits with abstract and concrete members
trait Showable:
  def show: String

trait Printable extends Showable:
  def printIt(): Unit = println(show)

trait Combinable[A]:
  def combine(other: A): A

class ShowInt(val value: Int) extends Printable with Combinable[ShowInt]:
  def show: String = value.toString
  def combine(other: ShowInt): ShowInt = ShowInt(value + other.value)

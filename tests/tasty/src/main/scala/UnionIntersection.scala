// Union and intersection types
object UnionIntersection:
  type StringOrInt = String | Int

  def stringify(x: StringOrInt): String = x match
    case s: String => s
    case i: Int => i.toString

  trait HasName:
    def name: String

  trait HasAge:
    def age: Int

  type Person = HasName & HasAge

  def describe(p: Person): String =
    s"${p.name}, age ${p.age}"

@main def multiFiles(): Unit =
  val items = List(Data("alpha", 10), Data("beta", 30), Data("gamma", 20))
  println(Utils.summarize(items))
  println(s"Max: ${Utils.maxByValue(items)}")

import matplotlib.pyplot as plt
import sys

plt.style.use("ggplot")

data = []
for line in sys.stdin.readlines():
    data.append(tuple(map(float, line.split(','))))

# plt.xscale('log')
plt.axis("off")
plt.plot([x for (x, y) in data], [0 for _ in data])
plt.bar([x for (x, y) in data], [y for (x, y) in data], 0.1)
plt.show()

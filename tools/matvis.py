import matplotlib.pyplot as plt

a = []
b = []
out = []

for i in range(0, 4):
    for k in range(0, 4):
        for j in range(0, 4):
            a.append((i, k))
            b.append((k, j))
            out.append((i, j))

fig = plt.figure()
ax1 = fig.add_subplot(131)
ax1.set_ylim(3, 0)
ax1.plot([y for (x,y) in a], [x for (x,y) in a])

ax2 = fig.add_subplot(132)
ax2.set_ylim(3, 0)
ax2.plot([y for (x,y) in b], [x for (x,y) in b])
print b

ax3 = fig.add_subplot(133)
ax3.set_ylim(3, 0)
ax3.plot([y for (x,y) in out], [x for (x,y) in out])

plt.show()

function sendPostRequest(url, data) {
  return fetch(url, {
    method: "POST",
    body: JSON.stringify(data),
    headers: {
      "Content-Type": "application/json",
    },
  });
}

const Toast = Swal.mixin({
  toast: true,
  position: "bottom-end",
  showConfirmButton: false,
  timer: 3000,
  timerProgressBar: true,
  didOpen: (toast) => {
    toast.onmouseenter = Swal.stopTimer;
    toast.onmouseleave = Swal.resumeTimer;
  }
});


document.addEventListener('swiped-left', function (e) {
  calendar.next();
});

document.addEventListener('swiped-right', function (e) {
  calendar.prev();
});

window.mobilecheck = function () {
  var check = false;
  (function (a) { if (/(android|bb\d+|meego).+mobile|avantgo|bada\/|blackberry|blazer|compal|elaine|fennec|hiptop|iemobile|ip(hone|od)|iris|kindle|lge |maemo|midp|mmp|mobile.+firefox|netfront|opera m(ob|in)i|palm( os)?|phone|p(ixi|re)\/|plucker|pocket|psp|series(4|6)0|symbian|treo|up\.(browser|link)|vodafone|wap|windows ce|xda|xiino/i.test(a) || /1207|6310|6590|3gso|4thp|50[1-6]i|770s|802s|a wa|abac|ac(er|oo|s\-)|ai(ko|rn)|al(av|ca|co)|amoi|an(ex|ny|yw)|aptu|ar(ch|go)|as(te|us)|attw|au(di|\-m|r |s )|avan|be(ck|ll|nq)|bi(lb|rd)|bl(ac|az)|br(e|v)w|bumb|bw\-(n|u)|c55\/|capi|ccwa|cdm\-|cell|chtm|cldc|cmd\-|co(mp|nd)|craw|da(it|ll|ng)|dbte|dc\-s|devi|dica|dmob|do(c|p)o|ds(12|\-d)|el(49|ai)|em(l2|ul)|er(ic|k0)|esl8|ez([4-7]0|os|wa|ze)|fetc|fly(\-|_)|g1 u|g560|gene|gf\-5|g\-mo|go(\.w|od)|gr(ad|un)|haie|hcit|hd\-(m|p|t)|hei\-|hi(pt|ta)|hp( i|ip)|hs\-c|ht(c(\-| |_|a|g|p|s|t)|tp)|hu(aw|tc)|i\-(20|go|ma)|i230|iac( |\-|\/)|ibro|idea|ig01|ikom|im1k|inno|ipaq|iris|ja(t|v)a|jbro|jemu|jigs|kddi|keji|kgt( |\/)|klon|kpt |kwc\-|kyo(c|k)|le(no|xi)|lg( g|\/(k|l|u)|50|54|\-[a-w])|libw|lynx|m1\-w|m3ga|m50\/|ma(te|ui|xo)|mc(01|21|ca)|m\-cr|me(rc|ri)|mi(o8|oa|ts)|mmef|mo(01|02|bi|de|do|t(\-| |o|v)|zz)|mt(50|p1|v )|mwbp|mywa|n10[0-2]|n20[2-3]|n30(0|2)|n50(0|2|5)|n7(0(0|1)|10)|ne((c|m)\-|on|tf|wf|wg|wt)|nok(6|i)|nzph|o2im|op(ti|wv)|oran|owg1|p800|pan(a|d|t)|pdxg|pg(13|\-([1-8]|c))|phil|pire|pl(ay|uc)|pn\-2|po(ck|rt|se)|prox|psio|pt\-g|qa\-a|qc(07|12|21|32|60|\-[2-7]|i\-)|qtek|r380|r600|raks|rim9|ro(ve|zo)|s55\/|sa(ge|ma|mm|ms|ny|va)|sc(01|h\-|oo|p\-)|sdk\/|se(c(\-|0|1)|47|mc|nd|ri)|sgh\-|shar|sie(\-|m)|sk\-0|sl(45|id)|sm(al|ar|b3|it|t5)|so(ft|ny)|sp(01|h\-|v\-|v )|sy(01|mb)|t2(18|50)|t6(00|10|18)|ta(gt|lk)|tcl\-|tdg\-|tel(i|m)|tim\-|t\-mo|to(pl|sh)|ts(70|m\-|m3|m5)|tx\-9|up(\.b|g1|si)|utst|v400|v750|veri|vi(rg|te)|vk(40|5[0-3]|\-v)|vm40|voda|vulc|vx(52|53|60|61|70|80|81|83|85|98)|w3c(\-| )|webc|whit|wi(g |nc|nw)|wmlb|wonu|x700|yas\-|your|zeto|zte\-/i.test(a.substr(0, 4))) check = true; })(navigator.userAgent || navigator.vendor || window.opera);
  return check;
};


function switchPages(toPage) {
  let messageDiv = document.getElementById("notices-div");
  let calendarDiv = document.getElementById("calendar-div");
  let messageButton = document.getElementById("notices");
  let calendarButton = document.getElementById("calendar");

  switch (toPage) {
    case "notices":
      messageDiv.removeAttribute("hidden");
      messageButton.setAttribute("class", "active");
      calendarDiv.setAttribute("hidden", "");
      calendarButton.setAttribute("class", "");
      break;
    case "calendar":
      messageDiv.setAttribute("hidden", "");
      messageButton.setAttribute("class", "");
      calendarDiv.removeAttribute("hidden");
      calendarButton.setAttribute("class", "active");
      break;
  }
}

async function logout() {
  document.getElementById("name-plate").innerHTML = "";
  document.getElementById("login").innerHTML = "Login";
  document.getElementById("login").onclick = showLoginForm;
  let response = await fetch("/api/logout");

  if (response.status !== 200) {
    Toast.fire({
      icon: "error",
      title: "Logout failed",
    });
    return;
  }

  logged_in = false;

  Toast.fire({
    icon: "success",
    title: "Logout successful",
  });
}

async function showLoginForm() {
  return new Promise((resolve) => {
    let dialog = document.getElementById("login-dialog");
    dialog.showModal();

    document.getElementById("login-button").onclick = async () => {
      dialog.close();
      try {
        const response = await sendPostRequest("/api/login", {
          username: document.getElementById("username").value,
          password: document.getElementById("password").value,
        });

        if (response.status === 200) {
          // Handle the successful response here
          Toast.fire({
            icon: "success",
            title: "Login successful",
          });

          console.log("Login successful");

          const data = await response.json();
          document.getElementById("name-plate").innerHTML = "Room " + data.user.room;
          document.getElementById("login").innerHTML = "Logout";
          document.getElementById("login").onclick = logout;
          logged_in = true;
          username = data.user.username;
          room = data.user.room;
          resolve(true)
        } else {
          const errorText = await response.text();
          Toast.fire({
            icon: "error",
            title: "Login failed",
            text: errorText,
          });
          logged_in = false;
          resolve(false);
        }
      } catch (error) {
        Toast.fire({
          icon: "error",
          title: "Login failed",
          text: "Something went wrong",
        });
        logged_in = false;
        resolve(false)
      }
    }

    document.getElementById("cancel-button").onclick = () => {
      dialog.close();
      Toast.fire({
        icon: "error",
        title: "Login cancelled",
      });
      resolve(false);
    }

    dialog.addEventListener("keypress", (event) => {
      if (event.key === "Enter") {
        document.getElementById("login-button").click();
      }
    })

    dialog.addEventListener('cancel', (event) => {
      document.getElementById("cancel-button").click();

    });
  })
}

document.addEventListener("DOMContentLoaded", function () {
  var calendarEl = document.getElementById("calendar-div");
  calendar = new FullCalendar.Calendar(calendarEl, {
    initialView: window.mobilecheck() ? "timeGridDay" : "month",
    events: '/api/book/events',
    height: "100%",
    selectable: true,
    selectMirror: true,
    unselectAuto: false,
    eventClick: handle_event_click,
    weekNumbers: true,
    select: calendarSelect,
    selectMinDistance: 10,
    headerToolbar: {
      left: 'today',
      center: 'title',
      right: 'month,timeGridWeek,timeGridDay,prev,next'
    },
    views: {
      month: {
        type: 'dayGridMonth',
        buttonText: 'Month',
        dateClick: (info) => { calendar.changeView('timeGridDay', info.startStr) },
        dayMaxEventRows: 3,
        dayMaxEvents: true,
        eventTimeFormat: {
          hour: 'numeric',
          minute: '2-digit',
          omitZeroMinute: true,
        }
      },
      timeGridWeek: {
        type: 'timeGrid',
        allDaySlot: false,
        slotDuration: '00:15:00',
        slotLabelInterval: '01:00',
        buttonText: 'Week',
        nowIndicator: true,
        scrollTime: '14:00:00',
        slotLabelFormat: {
          week: 'numeric',
          hour: 'numeric',
          minute: '2-digit',
          omitZeroMinute: true,
        }
      },
      timeGridDay: {
        type: 'timeGrid',
        allDaySlot: false,
        slotDuration: '00:15:00',
        slotLabelInterval: '01:00',
        buttonText: 'Day',
        nowIndicator: true,
        scrollTime: '14:00:00',
        slotLabelFormat: {
          hour: 'numeric',
          minute: '2-digit',
          omitZeroMinute: true,
        }
      },
    },
    firstDay: 1,
    locale: "dk",
  });
  calendar.render();
});


async function calendarSelect(info) {
  if (!logged_in) {
    if (await showLoginForm() == false) {
      calendar.unselect();
      return;
    }
  }
  await newBooking(info);
}

async function newBooking(info) {
  return new Promise((resolve, reject) => {


    // fill in the form with information of the resources
    let dropdown = document.getElementById("resources-dropdown");
    getResources().then((resources) => {
      resources.sort();
      dropdown.innerHTML = "";
      resources.forEach((resource) => {
        let option = document.createElement("option");
        option.value = resource[0];
        option.innerHTML = resource[1];
        dropdown.appendChild(option);
      });
    })

    let start = document.getElementById("booking-start");
    let end = document.getElementById("booking-end");
    start.value = info.startStr.slice(0, -6);
    end.value = info.endStr.slice(0, -6);

    let dialog = document.getElementById("create-booking-dialog");
    dialog.showModal();

    dialog.addEventListener("close", (event) => {
      calendar.refetchEvents();
      calendar.unselect();
    });
    document.getElementById("create-booking-button").onclick = async () => {
      const response = await sendPostRequest("/api/book/new", {
        start_time: rfc3339(start.value),
        end_time: rfc3339(end.value),
        resource_name: dropdown.value,
      });

      if (response.status === 200) {
        Toast.fire({
          icon: "success",
          title: "Booking successful"
        });

        console.log("Booking successful");
      } else if (response.status === 401) {
        const errorText = await response.text();
        Toast.fire({
          icon: "error",
          title: "Booking failed",
          text: "You need to log in first",
          // text: errorText,
        });
      }
      else {
        const errorText = await response.text();
        Toast.fire({
          icon: "error",
          title: "Booking failed",
          text: errorText,
        })
      }
      resolve();
      dialog.close();
    }
    document.getElementById("cancel-booking-button").onclick = () => {
      dialog.close();
      resolve();
    }
  });
}

function onSubmit(token) {
  document.getElementById("demo-form").submit();
}

async function check_login() {
  let response = await fetch("/api/login");
  if (response.status === 202) {
    const data = await response.json();
    document.getElementById("name-plate").innerHTML = "Room " + data.user.room;
    document.getElementById("login").innerHTML = "Logout";
    document.getElementById("login").onclick = logout;
    username = data.user.username;
    room = data.user.room;
    logged_in = true;
  } else if (response.status === 200) { // 200 means not logged in
    document.getElementById("login").onclick = showLoginForm;
    document.getElementById("name-plate").innerHTML = "";
    document.getElementById("login").innerHTML = "Login";
    logged_in = false
  }
}

async function handle_event_click(info) {
  new Promise((resolve, reject) => {

    //First check that the event is owned by the user
    if (info.event.extendedProps.owner != room) {
      document.getElementById("delete-booking-button").setAttribute("hidden", "");
    } else {
      document.getElementById("delete-booking-button").removeAttribute("hidden");
    }


    let dialog = document.getElementById("delete-booking-dialog");
    document.getElementById("delete-booking-header").innerHTML = info.event.title;
    document.getElementById("change-booking-start").value = info.event.startStr.slice(0, -6);
    document.getElementById("change-booking-end").value = info.event.endStr.slice(0, -6);
    dialog.showModal();

    // document.getElementById("change-booking-button").onclick = async () => {
    //   dialog.close();
    //   const response = await sendPostRequest("/api/book/change", {
    //     start_time: rfc3339(info.event.start),
    //     end_time: rfc3339(info.event.end),
    //     resource_name: info.event.resource_name,
    //     booking_id: info.event.id,
    //   });

    //   if (response.status === 200) {
    //     Toast.fire({
    //       icon: "success",
    //       title: "Booking changed"
    //     });

    //     console.log("Booking changed");
    //     resolve();
    //   } else if (response.status === 401) {
    //     const errorText = await response.text();
    //     Toast.fire({
    //       icon: "error",
    //       title: "Booking failed",
    //       text: "You need to log in first",
    //       // text: errorText,
    //     });
    //     resolve();
    //   }
    // }

    document.getElementById("cancel-change-booking-button").onclick = () => {
      dialog.close();
      resolve();
    }

    document.getElementById("delete-booking-button").onclick = async () => {
      dialog.close();
      const response = await sendPostRequest("/api/book/delete", {
        id: info.event.id,
      });

      if (response.status === 200) {
        Toast.fire({
          icon: "success",
          title: "Booking deleted"
        });

        console.log("Booking deleted");
        calendar.refetchEvents();
        resolve();
      } else if (response.status === 401) {
        const errorText = await response.text();
        Toast.fire({
          icon: "error",
          title: "Booking failed",
          text: "You need to log in first",
          // text: errorText,
        });
        resolve();
      }
    }

    dialog.addEventListener("close", (event) => {
      document.getElementById("cancel-change-booking-button").click();
    })
  })
}

var logged_in = false;
var username = "";
var room = -1;
document.onload = check_login();
setInterval(async function () {
  await check_login();
}, 10000);

async function getResources() {
  const response = await fetch('api/book/resources');
  const resources = await response.json();
  // return a list of resource name strings
  let resourceNames = [];
  for (const [key, value] of Object.entries(resources)) {
    resourceNames.push([key, value.name]);
  }
  return resourceNames;
}

function rfc3339(d) {
  var d = new Date(d);
  function pad(n) {
    return n < 10 ? "0" + n : n;
  }

  function timezoneOffset(offset) {
    var sign;
    if (offset === 0) {
      return "Z";
    }
    sign = (offset > 0) ? "-" : "+";
    offset = Math.abs(offset);
    return sign + pad(Math.floor(offset / 60)) + ":" + pad(offset % 60);
  }

  return d.getFullYear() + "-" +
    pad(d.getMonth() + 1) + "-" +
    pad(d.getDate()) + "T" +
    pad(d.getHours()) + ":" +
    pad(d.getMinutes()) + ":" +
    pad(d.getSeconds()) +
    timezoneOffset(d.getTimezoneOffset());
}
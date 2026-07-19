// Set darkmode
document.getElementById('mode').addEventListener('click', () => {

    document.documentElement.classList.toggle('dark');
    localStorage.setItem('theme', document.documentElement.classList.contains('dark') ? 'dark' : 'light');

});